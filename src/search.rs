use crate::{config, index};
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

pub fn search_current_dir(
    query: &str,
    limit: usize,
    show_snippets: bool,
) -> Result<Vec<SearchResult>> {
    search_repo(Path::new("."), query, limit, show_snippets)
}

pub fn search_repo(
    root: &Path,
    query: &str,
    limit: usize,
    show_snippets: bool,
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
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
    results.truncate(limit);
    Ok(results)
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

pub fn query_terms(query: &str) -> Vec<String> {
    query
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .map(str::trim)
        .filter(|term| term.len() >= 2)
        .map(|term| term.to_ascii_lowercase())
        .collect()
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
