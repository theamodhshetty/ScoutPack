use serde_json::Value;
#[derive(Debug, Clone)]
pub struct Chunk {
    pub kind: String,
    pub name: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub to_path: String,
    pub symbol: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub name: String,
    pub command: String,
    pub source: String,
}

#[derive(Debug, Default)]
pub struct ChunkedFile {
    pub chunks: Vec<Chunk>,
    pub symbols: Vec<Symbol>,
    pub imports: Vec<Import>,
    pub commands: Vec<CommandInfo>,
}

pub fn chunk_file(path: &str, language: &str, text: &str) -> ChunkedFile {
    match language {
        "markdown" => chunk_markdown(text),
        "json" => chunk_json(path, text),
        "typescript" => chunk_typescript(path, text),
        "yaml" | "toml" => chunk_config(path, text),
        _ => fallback_file_chunk(path, text),
    }
}

fn chunk_markdown(text: &str) -> ChunkedFile {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return ChunkedFile::default();
    }

    let mut starts = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if line.starts_with('#') {
            starts.push(idx);
        }
    }

    if starts.is_empty() {
        return ChunkedFile {
            chunks: vec![Chunk {
                kind: "markdown".to_owned(),
                name: None,
                start_line: 1,
                end_line: lines.len(),
                text: text.to_owned(),
            }],
            ..ChunkedFile::default()
        };
    }

    let mut chunks = Vec::new();
    let mut heading_path: Vec<String> = Vec::new();
    for (pos, start) in starts.iter().enumerate() {
        let end = starts
            .get(pos + 1)
            .copied()
            .unwrap_or(lines.len())
            .saturating_sub(1);
        let heading = lines[*start].trim();
        let level = heading.chars().take_while(|ch| *ch == '#').count().max(1);
        heading_path.truncate(level.saturating_sub(1));
        heading_path.push(heading.trim_start_matches('#').trim().to_owned());
        let name = heading_path.join(" / ");
        chunks.push(Chunk {
            kind: "markdown-section".to_owned(),
            name: Some(name),
            start_line: start + 1,
            end_line: end + 1,
            text: lines[*start..=end].join("\n"),
        });
    }

    ChunkedFile {
        chunks,
        ..ChunkedFile::default()
    }
}

fn chunk_json(path: &str, text: &str) -> ChunkedFile {
    if !path.ends_with("package.json") {
        return fallback_file_chunk(path, text);
    }

    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return fallback_file_chunk(path, text);
    };

    let mut chunks = Vec::new();
    let mut commands = Vec::new();

    if let Some(scripts) = value.get("scripts").and_then(Value::as_object) {
        let script_text = serde_json::to_string_pretty(scripts).unwrap_or_default();
        chunks.push(Chunk {
            kind: "package-scripts".to_owned(),
            name: Some("scripts".to_owned()),
            start_line: find_line(text, "\"scripts\"").unwrap_or(1),
            end_line: find_block_end(text, "\"scripts\"").unwrap_or_else(|| text.lines().count()),
            text: script_text,
        });
        for (name, command) in scripts {
            if let Some(command) = command.as_str() {
                commands.push(CommandInfo {
                    name: name.to_owned(),
                    command: command.to_owned(),
                    source: path.to_owned(),
                });
            }
        }
    }

    let mut deps = serde_json::Map::new();
    for key in ["dependencies", "devDependencies"] {
        if let Some(value) = value.get(key) {
            deps.insert(key.to_owned(), value.clone());
        }
    }
    if !deps.is_empty() {
        chunks.push(Chunk {
            kind: "package-dependencies".to_owned(),
            name: Some("dependencies".to_owned()),
            start_line: 1,
            end_line: text.lines().count().max(1),
            text: serde_json::to_string_pretty(&deps).unwrap_or_default(),
        });
    }

    if chunks.is_empty() {
        chunks = fallback_file_chunk(path, text).chunks;
    }

    ChunkedFile {
        chunks,
        commands,
        ..ChunkedFile::default()
    }
}

fn chunk_typescript(path: &str, text: &str) -> ChunkedFile {
    let lines: Vec<&str> = text.lines().collect();
    let mut imports = Vec::new();
    let mut symbols = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let raw_line = *line;
        let trimmed = raw_line.trim();
        if let Some(import) = parse_import(trimmed) {
            imports.push(import);
        }
        if let Some((kind, name)) = parse_symbol(raw_line, path) {
            symbols.push(Symbol {
                name,
                kind,
                start_line: idx + 1,
                end_line: idx + 1,
            });
        }
    }

    if symbols.is_empty() {
        return ChunkedFile {
            chunks: fallback_line_windows(path, text, 80),
            imports,
            ..ChunkedFile::default()
        };
    }

    let mut chunks = Vec::new();
    for idx in 0..symbols.len() {
        let start = symbols[idx].start_line;
        let end = symbols
            .get(idx + 1)
            .map(|next| next.start_line.saturating_sub(1))
            .unwrap_or(lines.len())
            .max(start);
        symbols[idx].end_line = end;
        chunks.push(Chunk {
            kind: symbols[idx].kind.clone(),
            name: Some(symbols[idx].name.clone()),
            start_line: start,
            end_line: end,
            text: lines[start - 1..end].join("\n"),
        });
    }

    ChunkedFile {
        chunks,
        symbols,
        imports,
        ..ChunkedFile::default()
    }
}

fn chunk_config(path: &str, text: &str) -> ChunkedFile {
    let chunks = if text.lines().count() > 120 {
        fallback_line_windows(path, text, 80)
    } else {
        fallback_file_chunk(path, text).chunks
    };
    ChunkedFile {
        chunks,
        ..ChunkedFile::default()
    }
}

fn fallback_file_chunk(path: &str, text: &str) -> ChunkedFile {
    let kind = if path.ends_with(".md") {
        "markdown"
    } else {
        "file"
    };
    ChunkedFile {
        chunks: vec![Chunk {
            kind: kind.to_owned(),
            name: None,
            start_line: 1,
            end_line: text.lines().count().max(1),
            text: text.to_owned(),
        }],
        ..ChunkedFile::default()
    }
}

fn fallback_line_windows(_path: &str, text: &str, window: usize) -> Vec<Chunk> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < lines.len() {
        let end = usize::min(start + window, lines.len());
        chunks.push(Chunk {
            kind: "line-window".to_owned(),
            name: None,
            start_line: start + 1,
            end_line: end,
            text: lines[start..end].join("\n"),
        });
        start = end;
    }
    chunks
}

fn parse_import(line: &str) -> Option<Import> {
    if !line.starts_with("import ") {
        return None;
    }
    let marker = " from ";
    let target = if let Some(pos) = line.find(marker) {
        &line[pos + marker.len()..]
    } else {
        line.trim_start_matches("import").trim()
    };
    let to_path = target
        .trim()
        .trim_end_matches(';')
        .trim_matches('"')
        .trim_matches('\'')
        .to_owned();
    if to_path.is_empty() {
        None
    } else {
        Some(Import {
            to_path,
            symbol: None,
        })
    }
}

fn parse_symbol(line: &str, path: &str) -> Option<(String, String)> {
    let original = line;
    let is_top_level = !original.starts_with(' ') && !original.starts_with('\t');
    if !is_top_level {
        return None;
    }

    let mut text = original
        .trim()
        .trim_start_matches("export ")
        .trim_start_matches("default ")
        .trim();
    let kind;
    if text.starts_with("async function ") {
        text = text.trim_start_matches("async ");
    }
    if text.starts_with("function ") {
        kind = "function";
        text = text.trim_start_matches("function ").trim();
    } else if text.starts_with("interface ") {
        kind = "interface";
        text = text.trim_start_matches("interface ").trim();
    } else if text.starts_with("type ") {
        kind = "type";
        text = text.trim_start_matches("type ").trim();
    } else if text.starts_with("class ") {
        kind = "class";
        text = text.trim_start_matches("class ").trim();
    } else if text.starts_with("const ") || text.starts_with("let ") {
        kind = "value";
        text = text
            .trim_start_matches("const ")
            .trim_start_matches("let ")
            .trim();
    } else {
        return None;
    }

    let name = text
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
        .next()
        .unwrap_or("")
        .trim();
    if name.is_empty() {
        return None;
    }

    let inferred_kind = if path.ends_with("route.ts")
        && ["GET", "POST", "PUT", "PATCH", "DELETE"].contains(&name)
    {
        "route-handler"
    } else if name
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
        && kind == "function"
    {
        "component"
    } else {
        kind
    };

    Some((inferred_kind.to_owned(), name.to_owned()))
}

fn find_line(text: &str, needle: &str) -> Option<usize> {
    text.lines()
        .enumerate()
        .find_map(|(idx, line)| line.contains(needle).then_some(idx + 1))
}

fn find_block_end(text: &str, needle: &str) -> Option<usize> {
    let start = find_line(text, needle)?;
    let mut depth = 0i32;
    let mut seen_open = false;
    for (idx, line) in text.lines().enumerate().skip(start - 1) {
        for ch in line.chars() {
            if ch == '{' {
                depth += 1;
                seen_open = true;
            } else if ch == '}' {
                depth -= 1;
                if seen_open && depth <= 0 {
                    return Some(idx + 1);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_splits_by_heading() {
        let chunked = chunk_file("README.md", "markdown", "# Title\nText\n## Install\nRun");
        assert_eq!(chunked.chunks.len(), 2);
        assert_eq!(chunked.chunks[1].name.as_deref(), Some("Title / Install"));
    }

    #[test]
    fn package_json_extracts_scripts() {
        let text = r#"{"scripts":{"test":"vitest","lint":"eslint ."},"dependencies":{"next":"1"}}"#;
        let chunked = chunk_file("package.json", "json", text);
        assert_eq!(chunked.commands.len(), 2);
        assert!(chunked
            .chunks
            .iter()
            .any(|chunk| chunk.kind == "package-scripts"));
    }

    #[test]
    fn typescript_extracts_symbols() {
        let text = "import x from './x';\nexport function LoginPage() {\n return null\n}\n";
        let chunked = chunk_file("src/app/login/page.tsx", "typescript", text);
        assert_eq!(chunked.imports.len(), 1);
        assert_eq!(chunked.symbols[0].name, "LoginPage");
        assert_eq!(chunked.symbols[0].kind, "component");
    }

    #[test]
    fn typescript_ignores_local_variables_as_symbols() {
        let text = "export function requireAuth() {\n  const session = getSession();\n  const url = new URL('/');\n}\n";
        let chunked = chunk_file("src/middleware/auth.ts", "typescript", text);
        assert_eq!(chunked.symbols.len(), 1);
        assert_eq!(chunked.symbols[0].name, "requireAuth");
        assert_eq!(chunked.chunks[0].end_line, 4);
    }
}
