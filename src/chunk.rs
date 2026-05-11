use serde_json::Value;
use tree_sitter::{Node, Parser};

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
    chunk_typescript_tree_sitter(path, text)
        .unwrap_or_else(|| chunk_typescript_heuristic(path, text))
}

fn chunk_typescript_tree_sitter(path: &str, text: &str) -> Option<ChunkedFile> {
    let mut parser = Parser::new();
    let language = if path.ends_with(".tsx") {
        tree_sitter_typescript::LANGUAGE_TSX
    } else {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT
    };
    parser.set_language(&language.into()).ok()?;
    let tree = parser.parse(text, None)?;
    let root = tree.root_node();
    if root.has_error() {
        return None;
    }

    let mut imports = Vec::new();
    let mut symbols = Vec::new();
    let mut chunks = Vec::new();
    let mut cursor = root.walk();

    for node in root.named_children(&mut cursor) {
        if node.kind() == "import_statement" {
            if let Some(import) = import_from_node(node, text) {
                imports.push(import);
            }
            continue;
        }

        let Some((chunk_node, declaration_node, kind, name)) =
            declaration_from_node(node, path, text)
        else {
            continue;
        };
        let start_line = chunk_node.start_position().row + 1;
        let end_line = chunk_node.end_position().row + 1;
        symbols.push(Symbol {
            name: name.clone(),
            kind: kind.clone(),
            start_line,
            end_line,
        });
        chunks.push(Chunk {
            kind,
            name: Some(name),
            start_line,
            end_line,
            text: node_text(chunk_node, text),
        });

        if declaration_node.kind() == "lexical_declaration"
            || declaration_node.kind() == "variable_statement"
        {
            continue;
        }
    }

    if symbols.is_empty() {
        return Some(ChunkedFile {
            chunks: fallback_line_windows(path, text, 80),
            imports,
            ..ChunkedFile::default()
        });
    }

    Some(ChunkedFile {
        chunks,
        symbols,
        imports,
        ..ChunkedFile::default()
    })
}

fn chunk_typescript_heuristic(path: &str, text: &str) -> ChunkedFile {
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

fn import_from_node(node: Node<'_>, text: &str) -> Option<Import> {
    let string_node = find_descendant_by_kind(node, "string")?;
    let to_path = node_text(string_node, text)
        .trim()
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

fn declaration_from_node<'a>(
    node: Node<'a>,
    path: &str,
    text: &str,
) -> Option<(Node<'a>, Node<'a>, String, String)> {
    let declaration = if node.kind() == "export_statement" {
        node.child_by_field_name("declaration")
            .or_else(|| first_declaration_child(node))?
    } else {
        node
    };

    let name = declaration_name(declaration, text)?;
    let kind = declaration_kind(path, declaration.kind(), &name)?;
    Some((node, declaration, kind, name))
}

fn first_declaration_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let declaration = node
        .named_children(&mut cursor)
        .find(|child| declaration_kind("", child.kind(), "x").is_some());
    declaration
}

fn declaration_name(node: Node<'_>, text: &str) -> Option<String> {
    if matches!(
        node.kind(),
        "lexical_declaration" | "variable_statement" | "variable_declaration"
    ) {
        let declarator = find_descendant_by_kind(node, "variable_declarator")?;
        let name = declarator.child_by_field_name("name")?;
        return Some(node_text(name, text));
    }

    node.child_by_field_name("name")
        .map(|name| node_text(name, text))
}

fn declaration_kind(path: &str, node_kind: &str, name: &str) -> Option<String> {
    let base_kind = match node_kind {
        "function_declaration" | "generator_function_declaration" => "function",
        "class_declaration" => "class",
        "interface_declaration" => "interface",
        "type_alias_declaration" => "type",
        "lexical_declaration" | "variable_statement" | "variable_declaration" => "value",
        _ => return None,
    };

    let kind = if (path.ends_with("route.ts") || path.ends_with("route.tsx"))
        && ["GET", "POST", "PUT", "PATCH", "DELETE"].contains(&name)
    {
        "route-handler"
    } else if name
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
        && matches!(base_kind, "function" | "value")
    {
        "component"
    } else {
        base_kind
    };
    Some(kind.to_owned())
}

fn find_descendant_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == kind {
            return Some(child);
        }
        if let Some(found) = find_descendant_by_kind(child, kind) {
            return Some(found);
        }
    }
    None
}

fn node_text(node: Node<'_>, text: &str) -> String {
    text.get(node.byte_range()).unwrap_or_default().to_owned()
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

    #[test]
    fn typescript_extracts_exported_types_and_const_components() {
        let text = "export interface User { id: string }\nexport type Mode = 'dark' | 'light';\nexport const SettingsPanel = () => <div />;\n";
        let chunked = chunk_file("src/settings.tsx", "typescript", text);
        let names: Vec<_> = chunked
            .symbols
            .iter()
            .map(|symbol| (symbol.kind.as_str(), symbol.name.as_str()))
            .collect();
        assert!(names.contains(&("interface", "User")));
        assert!(names.contains(&("type", "Mode")));
        assert!(names.contains(&("component", "SettingsPanel")));
    }

    #[test]
    fn typescript_extracts_route_handlers() {
        let text = "export async function GET() {\n  return Response.json({ ok: true });\n}\n";
        let chunked = chunk_file("src/app/api/health/route.ts", "typescript", text);
        assert_eq!(chunked.symbols[0].name, "GET");
        assert_eq!(chunked.symbols[0].kind, "route-handler");
    }
}
