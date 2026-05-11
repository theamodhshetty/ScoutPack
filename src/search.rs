use crate::{config, index};
use anyhow::Result;
use rusqlite::{params, Connection};
use std::{cmp::Ordering, path::Path};

#[derive(Debug, Clone)]
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
    let mut score = base;
    if terms.iter().any(|term| path_lower.contains(term)) {
        score += ranking.path_match_boost;
    }
    if terms.iter().any(|term| name_lower.contains(term)) {
        score += ranking.symbol_match_boost;
    }
    if path_lower.contains("/app/") || path_lower.ends_with("middleware.ts") {
        score += 0.1;
    }
    score
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
