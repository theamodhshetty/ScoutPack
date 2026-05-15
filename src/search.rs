use crate::{config, index, semantic};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub path: String,
    pub kind: String,
    pub name: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub text: Option<String>,
    pub score: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Copy)]
pub struct SearchOptions {
    pub semantic: bool,
    pub semantic_alpha: f64,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            semantic: false,
            semantic_alpha: 0.45,
        }
    }
}

pub fn search_current_dir_with_options(
    query: &str,
    limit: usize,
    show_snippets: bool,
    options: SearchOptions,
) -> Result<Vec<SearchResult>> {
    search_repo_with_options(Path::new("."), query, limit, show_snippets, options)
}

pub fn search_repo(
    root: &Path,
    query: &str,
    limit: usize,
    show_snippets: bool,
) -> Result<Vec<SearchResult>> {
    search_repo_with_options(root, query, limit, show_snippets, SearchOptions::default())
}

pub fn search_repo_with_options(
    root: &Path,
    query: &str,
    limit: usize,
    show_snippets: bool,
    options: SearchOptions,
) -> Result<Vec<SearchResult>> {
    let conn = index::ensure_index(root)?;
    let config = config::load(root)?;
    let terms = query_terms(query);
    if terms.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let fts = terms
        .iter()
        .map(|term| format!("{term}*"))
        .collect::<Vec<_>>()
        .join(" OR ");

    let mut results = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT f.path, c.kind, c.name, c.start_line, c.end_line, c.text, bm25(chunks_fts) AS rank
         FROM chunks_fts
         JOIN chunks c ON chunks_fts.rowid = c.id
         JOIN files f ON c.file_id = f.id
         WHERE chunks_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![fts, (limit * 5).max(limit) as i64], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, f64>(6)?,
        ))
    })?;

    for row in rows {
        let (path, kind, name, start_line, end_line, text, rank) = row?;
        let score = score_result(&path, name.as_deref(), -rank, &terms, &config.ranking);
        let reason = reason_for(&path, name.as_deref(), &text, &terms);
        results.push(SearchResult {
            path,
            kind,
            name,
            start_line: start_line as usize,
            end_line: end_line as usize,
            text: show_snippets.then(|| trim_snippet(&text, 900)),
            score,
            reason,
        });
    }

    add_symbol_matches(&conn, &terms, &mut results, &config.ranking)?;
    if options.semantic {
        let semantic_results = semantic::semantic_search(
            &conn,
            query,
            limit.saturating_mul(5).max(limit),
            show_snippets,
        )?;
        merge_semantic_results(&mut results, semantic_results, options.semantic_alpha);
    }
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    results.truncate(limit);
    Ok(results)
}

fn merge_semantic_results(
    results: &mut Vec<SearchResult>,
    semantic_results: Vec<SearchResult>,
    alpha: f64,
) {
    for semantic_result in semantic_results {
        if let Some(existing) = results.iter_mut().find(|result| {
            result.path == semantic_result.path
                && result.start_line == semantic_result.start_line
                && result.end_line == semantic_result.end_line
        }) {
            existing.score = semantic::hybrid_score(existing.score, semantic_result.score, alpha);
            existing.reason = format!("{}; {}", existing.reason, semantic_result.reason);
        } else {
            results.push(SearchResult {
                score: semantic::hybrid_score(0.0, semantic_result.score, alpha),
                ..semantic_result
            });
        }
    }
}

pub fn enrich_with_import_neighbors(
    root: &Path,
    seeds: &[SearchResult],
    limit: usize,
    show_snippets: bool,
) -> Result<Vec<SearchResult>> {
    if seeds.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let conn = index::ensure_index(root)?;
    let seed_paths: Vec<_> = seeds.iter().map(|result| result.path.as_str()).collect();
    let imports = imports_for_paths(&conn, &seed_paths)?;
    let all_paths = indexed_paths(&conn)?;
    let mut neighbor_reasons: HashMap<String, Vec<String>> = HashMap::new();

    for (from_path, import_path) in imports {
        if let Some(resolved) = resolve_import(&from_path, &import_path, &all_paths) {
            if seed_paths.contains(&resolved.as_str()) {
                continue;
            }
            neighbor_reasons
                .entry(resolved)
                .or_default()
                .push(format!("imported by `{from_path}`"));
        }
    }

    let mut neighbors = Vec::new();
    for (path, reasons) in neighbor_reasons {
        if let Some(result) = best_chunk_for_path(&conn, &path, show_snippets)? {
            neighbors.push(SearchResult {
                score: result.score + source_file_boost(&path) + 0.2,
                reason: reasons.join(", "),
                ..result
            });
        }
    }

    neighbors.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    neighbors.truncate(limit);
    Ok(neighbors)
}

pub fn chunks_for_paths(
    root: &Path,
    paths: &[String],
    show_snippets: bool,
) -> Result<Vec<SearchResult>> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let conn = index::ensure_index(root)?;
    let mut results = Vec::new();
    for path in paths {
        if let Some(result) = best_chunk_for_path(&conn, path, show_snippets)? {
            results.push(SearchResult {
                score: result.score + 2.0,
                reason: "included from git diff".to_owned(),
                ..result
            });
        }
    }
    Ok(results)
}

pub fn expand_call_graph(
    root: &Path,
    seeds: &[SearchResult],
    depth: usize,
    limit: usize,
    show_snippets: bool,
) -> Result<Vec<SearchResult>> {
    if seeds.is_empty() || depth == 0 || limit == 0 {
        return Ok(Vec::new());
    }
    let conn = index::ensure_index(root)?;
    let mut frontier: HashSet<String> = seeds
        .iter()
        .filter_map(|result| result.name.clone())
        .collect();
    let mut seen = frontier.clone();
    let mut expanded = Vec::new();

    for hop in 1..=depth.min(5) {
        if frontier.is_empty() {
            break;
        }
        let edges = call_edges_from_symbols(&conn, &frontier)?;
        let mut next = HashSet::new();
        for edge in edges {
            if !seen.insert(edge.to_symbol.clone()) {
                continue;
            }
            for result in chunks_for_symbol(&conn, &edge.to_symbol, show_snippets)? {
                expanded.push(SearchResult {
                    score: 1.0 - (hop as f64 * 0.1),
                    reason: format!(
                        "call graph: `{}` calls `{}` at {}:{}",
                        edge.from_symbol, edge.to_symbol, edge.from_path, edge.line
                    ),
                    ..result
                });
            }
            next.insert(edge.to_symbol);
        }
        frontier = next;
    }

    expanded.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    expanded.dedup_by(|a, b| {
        a.path == b.path && a.start_line == b.start_line && a.end_line == b.end_line
    });
    expanded.truncate(limit);
    Ok(expanded)
}

pub fn query_terms(query: &str) -> Vec<String> {
    query
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .map(str::trim)
        .filter(|term| term.len() >= 2)
        .map(|term| term.to_ascii_lowercase())
        .collect()
}

#[derive(Debug)]
struct CallEdgeRow {
    from_path: String,
    from_symbol: String,
    to_symbol: String,
    line: usize,
}

fn call_edges_from_symbols(
    conn: &Connection,
    symbols: &HashSet<String>,
) -> Result<Vec<CallEdgeRow>> {
    let mut stmt = conn.prepare(
        "SELECT f.path, e.from_symbol, e.to_symbol, e.line
         FROM symbol_edges e
         JOIN files f ON e.from_file_id = f.id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(CallEdgeRow {
            from_path: row.get(0)?,
            from_symbol: row.get(1)?,
            to_symbol: row.get(2)?,
            line: row.get::<_, i64>(3)? as usize,
        })
    })?;
    let mut edges = Vec::new();
    for row in rows {
        let edge = row?;
        if symbols.contains(&edge.from_symbol) {
            edges.push(edge);
        }
    }
    Ok(edges)
}

fn chunks_for_symbol(
    conn: &Connection,
    symbol: &str,
    show_snippets: bool,
) -> Result<Vec<SearchResult>> {
    let mut stmt = conn.prepare(
        "SELECT f.path, s.kind, s.name, s.start_line, s.end_line, c.text
         FROM symbols s
         JOIN files f ON s.file_id = f.id
         LEFT JOIN chunks c
           ON c.file_id = s.file_id
          AND c.start_line <= s.start_line
          AND c.end_line >= s.end_line
         WHERE s.name = ?1
         ORDER BY f.path, s.start_line
         LIMIT 8",
    )?;
    let rows = stmt.query_map(params![symbol], |row| {
        let text = row.get::<_, Option<String>>(5)?;
        Ok(SearchResult {
            path: row.get(0)?,
            kind: row.get(1)?,
            name: Some(row.get(2)?),
            start_line: row.get::<_, i64>(3)? as usize,
            end_line: row.get::<_, i64>(4)? as usize,
            text: text.and_then(|text| show_snippets.then(|| trim_snippet(&text, 900))),
            score: 0.0,
            reason: String::new(),
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn add_symbol_matches(
    conn: &Connection,
    terms: &[String],
    results: &mut Vec<SearchResult>,
    ranking: &config::RankingConfig,
) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT f.path, s.kind, s.name, s.start_line, s.end_line
         FROM symbols s
         JOIN files f ON s.file_id = f.id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;

    for row in rows {
        let (path, kind, name, start_line, end_line) = row?;
        let lower = name.to_ascii_lowercase();
        if terms.iter().any(|term| lower.contains(term)) {
            let score =
                score_result(&path, Some(&name), 1.0, terms, ranking) + ranking.symbol_match_boost;
            if !results.iter().any(|result| {
                result.path == path
                    && result.start_line == start_line as usize
                    && result.name.as_deref() == Some(&name)
            }) {
                results.push(SearchResult {
                    path,
                    kind,
                    name: Some(name),
                    start_line: start_line as usize,
                    end_line: end_line as usize,
                    text: None,
                    score,
                    reason: "matched symbol name".to_owned(),
                });
            }
        }
    }
    Ok(())
}

fn score_result(
    path: &str,
    name: Option<&str>,
    base: f64,
    terms: &[String],
    ranking: &config::RankingConfig,
) -> f64 {
    let path_lower = path.to_ascii_lowercase();
    let name_lower = name.unwrap_or_default().to_ascii_lowercase();
    let mut score = base.clamp(-0.25, 0.25);
    score += source_file_boost(path);
    score += kind_boost(name, &name_lower);

    let path_hits = terms
        .iter()
        .filter(|term| path_lower.contains(term.as_str()))
        .count();
    if path_hits > 0 {
        score += ranking.path_match_boost + (path_hits as f64 * 0.15);
    }
    let symbol_hits = terms
        .iter()
        .filter(|term| name_lower.contains(term.as_str()))
        .count();
    if symbol_hits > 0 {
        score += ranking.symbol_match_boost + (symbol_hits as f64 * 0.25);
    }
    if path_lower.contains("/app/") || path_lower.ends_with("middleware.ts") {
        score += 0.2;
    }
    if is_doc_path(&path_lower)
        && !terms
            .iter()
            .any(|term| matches!(term.as_str(), "doc" | "docs" | "readme"))
    {
        score -= 1.4;
    }
    score
}

fn source_file_boost(path: &str) -> f64 {
    let lower = path.to_ascii_lowercase();
    if matches!(
        Path::new(&lower).extension().and_then(|ext| ext.to_str()),
        Some("ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs")
    ) {
        1.2
    } else if lower.ends_with("package.json") {
        0.1
    } else {
        0.0
    }
}

fn kind_boost(name: Option<&str>, name_lower: &str) -> f64 {
    if name.is_none() {
        return 0.0;
    }
    if name_lower.is_empty() {
        0.0
    } else {
        0.2
    }
}

fn is_doc_path(path_lower: &str) -> bool {
    path_lower.ends_with(".md") || path_lower.contains("/docs/")
}

fn reason_for(path: &str, name: Option<&str>, text: &str, terms: &[String]) -> String {
    let haystacks = [
        ("path", path.to_ascii_lowercase()),
        ("symbol", name.unwrap_or_default().to_ascii_lowercase()),
        ("text", text.to_ascii_lowercase()),
    ];
    let mut reasons = Vec::new();
    for term in terms {
        for (label, haystack) in &haystacks {
            if haystack.contains(term) {
                reasons.push(format!("matched {term} in {label}"));
                break;
            }
        }
    }
    if reasons.is_empty() {
        "matched FTS query".to_owned()
    } else {
        reasons.join(", ")
    }
}

fn trim_snippet(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let mut out = text.chars().take(max_chars).collect::<String>();
    out.push_str("\n...");
    out
}

fn imports_for_paths(conn: &Connection, paths: &[&str]) -> Result<Vec<(String, String)>> {
    let path_set: HashSet<_> = paths.iter().copied().collect();
    let mut stmt = conn.prepare(
        "SELECT f.path, i.to_path
         FROM imports i
         JOIN files f ON i.from_file_id = f.id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut imports = Vec::new();
    for row in rows {
        let (from_path, to_path) = row?;
        if path_set.contains(from_path.as_str()) {
            imports.push((from_path, to_path));
        }
    }
    Ok(imports)
}

fn indexed_paths(conn: &Connection) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT path FROM files")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect::<rusqlite::Result<HashSet<_>>>()
        .map_err(Into::into)
}

fn best_chunk_for_path(
    conn: &Connection,
    path: &str,
    show_snippets: bool,
) -> Result<Option<SearchResult>> {
    let mut stmt = conn.prepare(
        "SELECT f.path, c.kind, c.name, c.start_line, c.end_line, c.text
         FROM chunks c
         JOIN files f ON c.file_id = f.id
         WHERE f.path = ?1
         ORDER BY CASE c.kind
           WHEN 'function' THEN 0
           WHEN 'component' THEN 1
           WHEN 'route-handler' THEN 2
           WHEN 'type' THEN 3
           WHEN 'interface' THEN 4
           ELSE 5
         END, c.start_line
         LIMIT 1",
    )?;
    let result = stmt
        .query_row(params![path], |row| {
            let text = row.get::<_, String>(5)?;
            Ok(SearchResult {
                path: row.get(0)?,
                kind: row.get(1)?,
                name: row.get(2)?,
                start_line: row.get::<_, i64>(3)? as usize,
                end_line: row.get::<_, i64>(4)? as usize,
                text: show_snippets.then(|| trim_snippet(&text, 900)),
                score: 0.35,
                reason: "import neighbor".to_owned(),
            })
        })
        .optional()?;
    Ok(result)
}

fn resolve_import(
    from_path: &str,
    import_path: &str,
    indexed_paths: &HashSet<String>,
) -> Option<String> {
    if !(import_path.starts_with("./") || import_path.starts_with("../")) {
        return None;
    }

    let from_dir = Path::new(from_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let normalized = normalize_path(&from_dir.join(import_path));
    let candidates = [
        normalized.clone(),
        format!("{normalized}.ts"),
        format!("{normalized}.tsx"),
        format!("{normalized}.js"),
        format!("{normalized}.jsx"),
        format!("{normalized}.mjs"),
        format!("{normalized}.cjs"),
        format!("{normalized}.json"),
        format!("{normalized}/index.ts"),
        format!("{normalized}/index.tsx"),
        format!("{normalized}/index.js"),
        format!("{normalized}/index.jsx"),
    ];
    candidates
        .into_iter()
        .find(|candidate| indexed_paths.contains(candidate))
}

fn normalize_path(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::ParentDir => {
                parts.pop();
            }
            Component::CurDir => {}
            _ => {}
        }
    }
    PathBuf::from_iter(parts)
        .to_string_lossy()
        .replace('\\', "/")
}
