use serde_json::Value;
use std::path::Path;
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
        "javascript" => chunk_javascript(path, text),
        "typescript" => chunk_typescript(path, text),
        "python" => chunk_python(path, text),
        "rust" => chunk_rust(path, text),
        "go" => chunk_go(path, text),
        "solidity" => chunk_solidity(path, text),
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

fn chunk_javascript(path: &str, text: &str) -> ChunkedFile {
    chunk_javascript_tree_sitter(path, text)
        .unwrap_or_else(|| chunk_typescript_heuristic(path, text))
}

fn chunk_javascript_tree_sitter(path: &str, text: &str) -> Option<ChunkedFile> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_javascript::LANGUAGE.into())
        .ok()?;
    let tree = parser.parse(text, None)?;
    let root = tree.root_node();
    if root.has_error() {
        return None;
    }

    let mut imports = commonjs_imports(text);
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

        if let Some((kind, name)) = javascript_route_call(node, text) {
            push_symbol_chunk(&mut symbols, &mut chunks, node, kind, name, text);
            continue;
        }

        let Some((chunk_node, declaration_node, kind, name)) =
            javascript_declaration_from_node(node, path, text)
        else {
            continue;
        };
        push_symbol_chunk(&mut symbols, &mut chunks, chunk_node, kind, name, text);

        if declaration_node.kind() == "lexical_declaration"
            || declaration_node.kind() == "variable_declaration"
        {
            continue;
        }
    }

    imports.sort_by(|a, b| a.to_path.cmp(&b.to_path));
    imports.dedup_by(|a, b| a.to_path == b.to_path && a.symbol == b.symbol);

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

fn javascript_declaration_from_node<'a>(
    node: Node<'a>,
    path: &str,
    text: &str,
) -> Option<(Node<'a>, Node<'a>, String, String)> {
    let declaration = if node.kind() == "export_statement" {
        node.child_by_field_name("declaration")
            .or_else(|| first_javascript_declaration_child(node))?
    } else {
        node
    };

    let name = javascript_declaration_name(declaration, text)?;
    let kind = javascript_declaration_kind(path, declaration, &name)?;
    Some((node, declaration, kind, name))
}

fn first_javascript_declaration_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let declaration = node.named_children(&mut cursor).find(|child| {
        matches!(
            child.kind(),
            "function_declaration"
                | "generator_function_declaration"
                | "class_declaration"
                | "lexical_declaration"
                | "variable_declaration"
        )
    });
    declaration
}

fn javascript_declaration_name(node: Node<'_>, text: &str) -> Option<String> {
    if matches!(node.kind(), "lexical_declaration" | "variable_declaration") {
        let declarator = find_descendant_by_kind(node, "variable_declarator")?;
        let name = declarator.child_by_field_name("name")?;
        return Some(node_text(name, text));
    }

    node.child_by_field_name("name")
        .map(|name| node_text(name, text))
}

fn javascript_declaration_kind(path: &str, node: Node<'_>, name: &str) -> Option<String> {
    let base_kind = match node.kind() {
        "function_declaration" | "generator_function_declaration" => "function",
        "class_declaration" => "class",
        "lexical_declaration" | "variable_declaration" => {
            if find_descendant_by_kind(node, "arrow_function").is_some()
                || find_descendant_by_kind(node, "function_expression").is_some()
            {
                "function"
            } else {
                "value"
            }
        }
        _ => return None,
    };

    let kind = if is_javascript_route_path(path) && HTTP_METHODS.contains(&name) {
        "route-handler"
    } else if name
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
        && matches!(base_kind, "function" | "value")
        && (path.ends_with(".jsx")
            || find_descendant_by_kind(node, "jsx_element").is_some()
            || find_descendant_by_kind(node, "jsx_self_closing_element").is_some())
    {
        "component"
    } else {
        base_kind
    };
    Some(kind.to_owned())
}

const HTTP_METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE"];

fn is_javascript_route_path(path: &str) -> bool {
    matches!(
        Path::new(path).file_name().and_then(|name| name.to_str()),
        Some("route.js" | "route.jsx" | "route.mjs" | "route.cjs")
    )
}

fn javascript_route_call(node: Node<'_>, text: &str) -> Option<(String, String)> {
    if node.kind() != "expression_statement" {
        return None;
    }
    let raw = node_text(node, text);
    let method = [
        ("get", "GET"),
        ("post", "POST"),
        ("put", "PUT"),
        ("patch", "PATCH"),
        ("delete", "DELETE"),
    ]
    .iter()
    .find_map(|(needle, label)| (raw.contains(&format!(".{needle}("))).then_some(*label))?;
    let route = quoted_strings(&raw)
        .into_iter()
        .next()
        .unwrap_or_else(|| "unknown".to_owned());
    Some(("route-handler".to_owned(), format!("{method} {route}")))
}

fn commonjs_imports(text: &str) -> Vec<Import> {
    let mut imports = Vec::new();
    for line in text.lines() {
        if !line.contains("require(") {
            continue;
        }
        for import in quoted_strings(line) {
            imports.push(Import {
                to_path: import,
                symbol: None,
            });
        }
    }
    imports
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

fn chunk_python(path: &str, text: &str) -> ChunkedFile {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .is_err()
    {
        return ChunkedFile {
            chunks: fallback_line_windows(path, text, 80),
            ..ChunkedFile::default()
        };
    }
    let Some(tree) = parser.parse(text, None) else {
        return ChunkedFile {
            chunks: fallback_line_windows(path, text, 80),
            ..ChunkedFile::default()
        };
    };
    let root = tree.root_node();
    if root.has_error() {
        return ChunkedFile {
            chunks: fallback_line_windows(path, text, 80),
            ..ChunkedFile::default()
        };
    }

    let mut imports = Vec::new();
    let mut symbols = Vec::new();
    let mut chunks = Vec::new();
    let mut cursor = root.walk();
    for node in root.named_children(&mut cursor) {
        match node.kind() {
            "import_statement" | "import_from_statement" => {
                if let Some(import) = python_import_from_node(node, text) {
                    imports.push(import);
                }
            }
            "function_definition" | "class_definition" | "decorated_definition" => {
                if let Some((chunk_node, kind, name)) = python_symbol_from_node(node, text) {
                    push_symbol_chunk(&mut symbols, &mut chunks, chunk_node, kind, name, text);
                }
            }
            _ => {}
        }
    }

    if chunks.is_empty() {
        chunks = fallback_line_windows(path, text, 80);
    }
    ChunkedFile {
        chunks,
        symbols,
        imports,
        ..ChunkedFile::default()
    }
}

fn python_import_from_node(node: Node<'_>, text: &str) -> Option<Import> {
    let raw = node_text(node, text);
    let trimmed = raw.trim();
    let to_path = if let Some(rest) = trimmed.strip_prefix("from ") {
        rest.split_whitespace().next().unwrap_or("").to_owned()
    } else if let Some(rest) = trimmed.strip_prefix("import ") {
        rest.split(',')
            .next()
            .unwrap_or("")
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_owned()
    } else {
        String::new()
    };
    if to_path.is_empty() {
        None
    } else {
        Some(Import {
            to_path,
            symbol: None,
        })
    }
}

fn python_symbol_from_node<'a>(node: Node<'a>, text: &str) -> Option<(Node<'a>, String, String)> {
    let definition = if node.kind() == "decorated_definition" {
        let mut cursor = node.walk();
        let definition = node
            .named_children(&mut cursor)
            .find(|child| matches!(child.kind(), "function_definition" | "class_definition"))?;
        definition
    } else {
        node
    };
    let name = definition
        .child_by_field_name("name")
        .map(|name| node_text(name, text))?;
    let kind = if definition.kind() == "class_definition" {
        "class"
    } else if node.kind() == "decorated_definition" && is_python_route(node, text) {
        "route-handler"
    } else {
        "function"
    };
    Some((node, kind.to_owned(), name))
}

fn is_python_route(node: Node<'_>, text: &str) -> bool {
    node_text(node, text).lines().take(8).any(|line| {
        let line = line.trim();
        line.starts_with("@app.")
            || line.starts_with("@router.")
            || line.contains(".get(")
            || line.contains(".post(")
            || line.contains(".put(")
            || line.contains(".patch(")
            || line.contains(".delete(")
    })
}

fn chunk_rust(path: &str, text: &str) -> ChunkedFile {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .is_err()
    {
        return ChunkedFile {
            chunks: fallback_line_windows(path, text, 80),
            ..ChunkedFile::default()
        };
    }
    let Some(tree) = parser.parse(text, None) else {
        return ChunkedFile {
            chunks: fallback_line_windows(path, text, 80),
            ..ChunkedFile::default()
        };
    };
    let root = tree.root_node();
    if root.has_error() {
        return ChunkedFile {
            chunks: fallback_line_windows(path, text, 80),
            ..ChunkedFile::default()
        };
    }

    let mut imports = Vec::new();
    let mut symbols = Vec::new();
    let mut chunks = Vec::new();
    let mut cursor = root.walk();
    for node in root.named_children(&mut cursor) {
        match node.kind() {
            "use_declaration" => {
                if let Some(import) = rust_import_from_node(node, text) {
                    imports.push(import);
                }
            }
            "function_item" | "struct_item" | "enum_item" | "trait_item" | "impl_item"
            | "mod_item" => {
                if let Some((kind, name)) = rust_symbol_from_node(node, text) {
                    push_symbol_chunk(&mut symbols, &mut chunks, node, kind, name, text);
                }
            }
            _ => {}
        }
    }

    if chunks.is_empty() {
        chunks = fallback_line_windows(path, text, 80);
    }
    ChunkedFile {
        chunks,
        symbols,
        imports,
        ..ChunkedFile::default()
    }
}

fn rust_import_from_node(node: Node<'_>, text: &str) -> Option<Import> {
    let to_path = node_text(node, text)
        .trim()
        .trim_start_matches("use")
        .trim()
        .trim_end_matches(';')
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

fn rust_symbol_from_node(node: Node<'_>, text: &str) -> Option<(String, String)> {
    let kind = match node.kind() {
        "function_item" => "function",
        "struct_item" => "struct",
        "enum_item" => "enum",
        "trait_item" => "trait",
        "impl_item" => "impl",
        "mod_item" => "module",
        _ => return None,
    };
    let name = if node.kind() == "impl_item" {
        let type_node = find_descendant_by_kind(node, "type_identifier")
            .or_else(|| find_descendant_by_kind(node, "generic_type"))?;
        format!("impl {}", node_text(type_node, text))
    } else {
        node.child_by_field_name("name")
            .map(|name| node_text(name, text))?
    };
    Some((kind.to_owned(), name))
}

fn chunk_go(path: &str, text: &str) -> ChunkedFile {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_go::LANGUAGE.into())
        .is_err()
    {
        return ChunkedFile {
            chunks: fallback_line_windows(path, text, 80),
            ..ChunkedFile::default()
        };
    }
    let Some(tree) = parser.parse(text, None) else {
        return ChunkedFile {
            chunks: fallback_line_windows(path, text, 80),
            ..ChunkedFile::default()
        };
    };
    let root = tree.root_node();
    if root.has_error() {
        return ChunkedFile {
            chunks: fallback_line_windows(path, text, 80),
            ..ChunkedFile::default()
        };
    }

    let mut imports = Vec::new();
    let mut symbols = Vec::new();
    let mut chunks = Vec::new();
    let mut cursor = root.walk();
    for node in root.named_children(&mut cursor) {
        match node.kind() {
            "package_clause" => {
                if let Some(name) = go_package_name(node, text) {
                    push_symbol_chunk(
                        &mut symbols,
                        &mut chunks,
                        node,
                        "package".to_owned(),
                        name,
                        text,
                    );
                }
            }
            "import_declaration" => {
                for import in quoted_strings(&node_text(node, text)) {
                    imports.push(Import {
                        to_path: import,
                        symbol: None,
                    });
                }
            }
            "function_declaration" | "method_declaration" => {
                if let Some(name) = node
                    .child_by_field_name("name")
                    .map(|name| node_text(name, text))
                {
                    let kind = if node.kind() == "method_declaration" {
                        "method"
                    } else {
                        "function"
                    };
                    push_symbol_chunk(&mut symbols, &mut chunks, node, kind.to_owned(), name, text);
                }
            }
            "type_declaration" => {
                let mut type_cursor = node.walk();
                for child in node.named_children(&mut type_cursor) {
                    if child.kind() == "type_spec" {
                        if let Some((kind, name)) = go_type_from_spec(child, text) {
                            push_symbol_chunk(&mut symbols, &mut chunks, child, kind, name, text);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if chunks.is_empty() {
        chunks = fallback_line_windows(path, text, 80);
    }
    ChunkedFile {
        chunks,
        symbols,
        imports,
        ..ChunkedFile::default()
    }
}

fn go_package_name(node: Node<'_>, text: &str) -> Option<String> {
    node_text(node, text)
        .trim()
        .strip_prefix("package")
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
}

fn go_type_from_spec(node: Node<'_>, text: &str) -> Option<(String, String)> {
    let name = node
        .child_by_field_name("name")
        .map(|name| node_text(name, text))?;
    let kind = if find_descendant_by_kind(node, "struct_type").is_some() {
        "struct"
    } else if find_descendant_by_kind(node, "interface_type").is_some() {
        "interface"
    } else {
        "type"
    };
    Some((kind.to_owned(), name))
}

fn chunk_solidity(path: &str, text: &str) -> ChunkedFile {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_solidity::LANGUAGE.into())
        .is_err()
    {
        return ChunkedFile {
            chunks: fallback_line_windows(path, text, 80),
            ..ChunkedFile::default()
        };
    }
    let Some(tree) = parser.parse(text, None) else {
        return ChunkedFile {
            chunks: fallback_line_windows(path, text, 80),
            ..ChunkedFile::default()
        };
    };
    let root = tree.root_node();
    if root.has_error() {
        return ChunkedFile {
            chunks: fallback_line_windows(path, text, 80),
            ..ChunkedFile::default()
        };
    }

    let mut imports = Vec::new();
    let mut symbols = Vec::new();
    let mut chunks = Vec::new();
    collect_solidity(root, text, &mut imports, &mut symbols, &mut chunks);

    if chunks.is_empty() {
        chunks = fallback_line_windows(path, text, 80);
    }
    ChunkedFile {
        chunks,
        symbols,
        imports,
        ..ChunkedFile::default()
    }
}

fn collect_solidity(
    node: Node<'_>,
    text: &str,
    imports: &mut Vec<Import>,
    symbols: &mut Vec<Symbol>,
    chunks: &mut Vec<Chunk>,
) {
    match node.kind() {
        "import_directive" => {
            for import in quoted_strings(&node_text(node, text)) {
                imports.push(Import {
                    to_path: import,
                    symbol: None,
                });
            }
        }
        "contract_declaration"
        | "interface_declaration"
        | "library_declaration"
        | "function_definition"
        | "modifier_definition"
        | "event_definition" => {
            if let Some((kind, name)) = solidity_symbol_from_node(node, text) {
                push_symbol_chunk(symbols, chunks, node, kind, name, text);
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_solidity(child, text, imports, symbols, chunks);
    }
}

fn solidity_symbol_from_node(node: Node<'_>, text: &str) -> Option<(String, String)> {
    let kind = match node.kind() {
        "contract_declaration" => "contract",
        "interface_declaration" => "interface",
        "library_declaration" => "library",
        "function_definition" => "function",
        "modifier_definition" => "modifier",
        "event_definition" => "event",
        _ => return None,
    };
    let name = node
        .child_by_field_name("name")
        .map(|name| node_text(name, text))
        .filter(|name| !name.is_empty())?;
    Some((kind.to_owned(), name))
}

fn quoted_strings(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((start, quote)) = chars.next() {
        if quote != '"' && quote != '\'' {
            continue;
        }
        for (end, ch) in chars.by_ref() {
            if ch == quote {
                let value = text[start + quote.len_utf8()..end].trim();
                if !value.is_empty() {
                    values.push(value.to_owned());
                }
                break;
            }
        }
    }
    values
}

fn push_symbol_chunk(
    symbols: &mut Vec<Symbol>,
    chunks: &mut Vec<Chunk>,
    node: Node<'_>,
    kind: String,
    name: String,
    text: &str,
) {
    let start_line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
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
        text: node_text(node, text),
    });
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
    fn javascript_extracts_esm_commonjs_functions_classes_and_components() {
        let text = "import express from 'express';\nconst bcrypt = require('bcryptjs');\n\nexport class AuthService {}\nexport function requireAuth(req, res, next) {\n  next();\n}\nexport const loginUser = async (req, res) => {\n  return res.json({ ok: true });\n};\nexport const LoginButton = () => <button>Login</button>;\n";
        let chunked = chunk_file("src/server.jsx", "javascript", text);
        let symbols: Vec<_> = chunked
            .symbols
            .iter()
            .map(|symbol| (symbol.kind.as_str(), symbol.name.as_str()))
            .collect();
        assert!(chunked
            .imports
            .iter()
            .any(|import| import.to_path == "express"));
        assert!(chunked
            .imports
            .iter()
            .any(|import| import.to_path == "bcryptjs"));
        assert!(symbols.contains(&("class", "AuthService")));
        assert!(symbols.contains(&("function", "requireAuth")));
        assert!(symbols.contains(&("function", "loginUser")));
        assert!(symbols.contains(&("component", "LoginButton")));
    }

    #[test]
    fn javascript_extracts_express_route_handlers() {
        let text = "const express = require('express');\nconst app = express();\n\nfunction requireAuth(req, res, next) {\n  next();\n}\n\napp.get('/auth/session', requireAuth);\napp.post('/login', (req, res) => res.sendStatus(204));\n";
        let chunked = chunk_file("src/server.js", "javascript", text);
        let symbols: Vec<_> = chunked
            .symbols
            .iter()
            .map(|symbol| (symbol.kind.as_str(), symbol.name.as_str()))
            .collect();
        assert!(symbols.contains(&("function", "requireAuth")));
        assert!(symbols.contains(&("route-handler", "GET /auth/session")));
        assert!(symbols.contains(&("route-handler", "POST /login")));
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

    #[test]
    fn python_extracts_functions_classes_routes_and_imports() {
        let text = "from fastapi import APIRouter\nimport services.auth\n\nrouter = APIRouter()\n\nclass UserService:\n    pass\n\n@router.get('/users/{user_id}')\ndef get_user(user_id: str):\n    return {'id': user_id}\n\ndef helper():\n    return None\n";
        let chunked = chunk_file("app/api/users.py", "python", text);
        let symbols: Vec<_> = chunked
            .symbols
            .iter()
            .map(|symbol| (symbol.kind.as_str(), symbol.name.as_str()))
            .collect();
        assert!(chunked
            .imports
            .iter()
            .any(|import| import.to_path == "fastapi"));
        assert!(chunked
            .imports
            .iter()
            .any(|import| import.to_path == "services.auth"));
        assert!(symbols.contains(&("class", "UserService")));
        assert!(symbols.contains(&("route-handler", "get_user")));
        assert!(symbols.contains(&("function", "helper")));
    }

    #[test]
    fn rust_extracts_items_impls_modules_and_imports() {
        let text = "use crate::config::Config;\n\npub mod commands;\n\npub struct Cli {\n    name: String,\n}\n\npub enum Mode {\n    Fast,\n}\n\npub trait Runnable {\n    fn run(&self);\n}\n\nimpl Cli {\n    pub fn new() -> Self {\n        Self { name: String::new() }\n    }\n}\n\npub fn execute() {}\n";
        let chunked = chunk_file("src/main.rs", "rust", text);
        let symbols: Vec<_> = chunked
            .symbols
            .iter()
            .map(|symbol| (symbol.kind.as_str(), symbol.name.as_str()))
            .collect();
        assert!(chunked
            .imports
            .iter()
            .any(|import| import.to_path == "crate::config::Config"));
        assert!(symbols.contains(&("module", "commands")));
        assert!(symbols.contains(&("struct", "Cli")));
        assert!(symbols.contains(&("enum", "Mode")));
        assert!(symbols.contains(&("trait", "Runnable")));
        assert!(symbols.contains(&("impl", "impl Cli")));
        assert!(symbols.contains(&("function", "execute")));
    }

    #[test]
    fn go_extracts_package_imports_functions_methods_and_types() {
        let text = "package api\n\nimport (\n    \"context\"\n    \"net/http\"\n)\n\ntype User struct {\n    ID string\n}\n\ntype Store interface {\n    Get(context.Context, string) (User, error)\n}\n\nfunc NewUser() User {\n    return User{}\n}\n\nfunc (u User) Validate() bool {\n    return u.ID != \"\"\n}\n";
        let chunked = chunk_file("internal/api/user.go", "go", text);
        let symbols: Vec<_> = chunked
            .symbols
            .iter()
            .map(|symbol| (symbol.kind.as_str(), symbol.name.as_str()))
            .collect();
        assert!(chunked
            .imports
            .iter()
            .any(|import| import.to_path == "context"));
        assert!(chunked
            .imports
            .iter()
            .any(|import| import.to_path == "net/http"));
        assert!(symbols.contains(&("package", "api")));
        assert!(symbols.contains(&("struct", "User")));
        assert!(symbols.contains(&("interface", "Store")));
        assert!(symbols.contains(&("function", "NewUser")));
        assert!(symbols.contains(&("method", "Validate")));
    }

    #[test]
    fn solidity_extracts_contracts_interfaces_libraries_functions_modifiers_events_and_imports() {
        let text = "import \"./Ownable.sol\";\n\ninterface IERC20 {\n    function transfer(address to, uint256 amount) external returns (bool);\n}\n\nlibrary SafeMath {\n    function add(uint256 a, uint256 b) internal pure returns (uint256) { return a + b; }\n}\n\ncontract Vault is Ownable {\n    event Withdraw(address indexed user, uint256 amount);\n\n    modifier onlyOwner() {\n        _;\n    }\n\n    function withdraw(uint256 amount) external onlyOwner {\n        emit Withdraw(msg.sender, amount);\n    }\n}\n";
        let chunked = chunk_file("contracts/Vault.sol", "solidity", text);
        let symbols: Vec<_> = chunked
            .symbols
            .iter()
            .map(|symbol| (symbol.kind.as_str(), symbol.name.as_str()))
            .collect();
        assert!(chunked
            .imports
            .iter()
            .any(|import| import.to_path == "./Ownable.sol"));
        assert!(symbols.contains(&("interface", "IERC20")));
        assert!(symbols.contains(&("library", "SafeMath")));
        assert!(symbols.contains(&("contract", "Vault")));
        assert!(symbols.contains(&("event", "Withdraw")));
        assert!(symbols.contains(&("modifier", "onlyOwner")));
        assert!(symbols.contains(&("function", "withdraw")));
    }
}
