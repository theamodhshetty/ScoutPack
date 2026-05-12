use crate::search::SearchResult;
use anyhow::Result;
use rusqlite::Connection;

#[cfg(feature = "semantic")]
pub const DEFAULT_MODEL: &str = "BAAI/bge-small-en-v1.5";

pub fn hybrid_score(keyword_score: f64, semantic_score: f64, alpha: f64) -> f64 {
    let alpha = alpha.clamp(0.0, 1.0);
    let keyword = normalize_keyword_score(keyword_score);
    let semantic = semantic_score.clamp(-1.0, 1.0);
    ((1.0 - alpha) * keyword) + (alpha * semantic)
}

fn normalize_keyword_score(score: f64) -> f64 {
    if score <= 0.0 {
        0.0
    } else {
        (score / 4.0).min(1.0)
    }
}

#[cfg(not(feature = "semantic"))]
pub fn embed_missing_chunks(_conn: &Connection, _allow_download: bool) -> Result<usize> {
    anyhow::bail!(
        "Semantic embeddings require a binary built with `--features semantic`. Reinstall with `cargo install --path . --features semantic` or use default FTS search."
    );
}

#[cfg(not(feature = "semantic"))]
pub fn semantic_search(
    _conn: &Connection,
    _query: &str,
    _limit: usize,
    _show_snippets: bool,
) -> Result<Vec<SearchResult>> {
    anyhow::bail!(
        "Semantic search requires a binary built with `--features semantic` and an index created with `scoutpack pack . --embed`."
    );
}

#[cfg(feature = "semantic")]
pub fn embed_missing_chunks(conn: &Connection, allow_download: bool) -> Result<usize> {
    semantic_impl::embed_missing_chunks(conn, allow_download)
}

#[cfg(feature = "semantic")]
pub fn semantic_search(
    conn: &Connection,
    query: &str,
    limit: usize,
    show_snippets: bool,
) -> Result<Vec<SearchResult>> {
    semantic_impl::semantic_search(conn, query, limit, show_snippets)
}

#[cfg(feature = "semantic")]
mod semantic_impl {
    use super::DEFAULT_MODEL;
    use crate::search::SearchResult;
    use anyhow::{Context, Result};
    use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
    use rusqlite::{params, Connection};
    use std::{
        env, fs,
        io::{self, Write},
        path::{Path, PathBuf},
    };

    pub fn embed_missing_chunks(conn: &Connection, allow_download: bool) -> Result<usize> {
        ensure_model_download_allowed(allow_download)?;
        let mut rows = read_unembedded_chunks(conn)?;
        if rows.is_empty() {
            return Ok(0);
        }

        let mut model = load_model()?;
        let embedded_count = rows.len();
        let texts = rows
            .iter()
            .map(|(_, text)| text.clone())
            .collect::<Vec<_>>();
        let embeddings = model
            .embed(texts, None)
            .context("Could not generate local semantic embeddings")?;

        let tx = conn.unchecked_transaction()?;
        for ((chunk_id, _), vector) in rows.drain(..).zip(embeddings) {
            let blob = encode_vector(&vector);
            tx.execute(
                "INSERT OR REPLACE INTO chunk_embeddings (chunk_id, model, dim, vector)
                 VALUES (?1, ?2, ?3, ?4)",
                params![chunk_id, DEFAULT_MODEL, vector.len() as i64, blob],
            )?;
        }
        tx.commit()?;
        Ok(embedded_count)
    }

    pub fn semantic_search(
        conn: &Connection,
        query: &str,
        limit: usize,
        show_snippets: bool,
    ) -> Result<Vec<SearchResult>> {
        if query.trim().is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let stored = read_embeddings(conn)?;
        if stored.is_empty() {
            anyhow::bail!("No semantic embeddings found. Run `scoutpack pack . --embed` first.");
        }

        ensure_model_download_allowed(false)?;
        let mut model = load_model()?;
        let query_embeddings = model
            .embed(vec![query], None)
            .context("Could not embed semantic query")?;
        let Some(query_vector) = query_embeddings.first() else {
            return Ok(Vec::new());
        };

        let mut scored = stored
            .into_iter()
            .map(|mut row| {
                row.score = cosine_similarity(query_vector, &row.embedding);
                row
            })
            .collect::<Vec<_>>();
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(limit);

        Ok(scored
            .into_iter()
            .map(|row| SearchResult {
                path: row.path,
                kind: row.kind,
                name: row.name,
                start_line: row.start_line,
                end_line: row.end_line,
                text: show_snippets.then(|| trim_snippet(&row.text, 900)),
                score: row.score,
                reason: format!(
                    "semantic similarity to task using local `{DEFAULT_MODEL}` embeddings"
                ),
            })
            .collect())
    }

    struct StoredEmbedding {
        path: String,
        kind: String,
        name: Option<String>,
        start_line: usize,
        end_line: usize,
        text: String,
        embedding: Vec<f32>,
        score: f64,
    }

    fn read_unembedded_chunks(conn: &Connection) -> Result<Vec<(i64, String)>> {
        let mut stmt = conn.prepare(
            "SELECT c.id, c.text
             FROM chunks c
             LEFT JOIN chunk_embeddings e
               ON e.chunk_id = c.id AND e.model = ?1
             WHERE e.chunk_id IS NULL
             ORDER BY c.id",
        )?;
        let rows = stmt.query_map(params![DEFAULT_MODEL], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn read_embeddings(conn: &Connection) -> Result<Vec<StoredEmbedding>> {
        let mut stmt = conn.prepare(
            "SELECT f.path, c.kind, c.name, c.start_line, c.end_line, c.text, e.vector
             FROM chunk_embeddings e
             JOIN chunks c ON e.chunk_id = c.id
             JOIN files f ON c.file_id = f.id
             WHERE e.model = ?1",
        )?;
        let rows = stmt.query_map(params![DEFAULT_MODEL], |row| {
            let blob = row.get::<_, Vec<u8>>(6)?;
            Ok(StoredEmbedding {
                path: row.get(0)?,
                kind: row.get(1)?,
                name: row.get(2)?,
                start_line: row.get::<_, i64>(3)? as usize,
                end_line: row.get::<_, i64>(4)? as usize,
                text: row.get(5)?,
                embedding: decode_vector(&blob),
                score: 0.0,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn load_model() -> Result<TextEmbedding> {
        let cache_dir = model_cache_dir()?;
        let options = TextInitOptions::new(EmbeddingModel::BGESmallENV15)
            .with_cache_dir(cache_dir)
            .with_show_download_progress(true);
        TextEmbedding::try_new(options).context("Could not load local embedding model")
    }

    fn ensure_model_download_allowed(allow_download: bool) -> Result<()> {
        let cache_dir = model_cache_dir()?;
        if cache_has_files(&cache_dir) {
            return Ok(());
        }
        if allow_download || confirm_download(&cache_dir)? {
            return Ok(());
        }
        anyhow::bail!(
            "Semantic model download declined. Run again with `--allow-model-download` or pre-populate {}.",
            cache_dir.display()
        );
    }

    fn model_cache_dir() -> Result<PathBuf> {
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .context("HOME is not set; cannot locate ScoutPack model cache")?;
        Ok(home.join(".cache/scoutpack/models"))
    }

    fn cache_has_files(path: &PathBuf) -> bool {
        fs::read_dir(path)
            .ok()
            .and_then(|mut entries| entries.next())
            .is_some()
    }

    fn confirm_download(cache_dir: &Path) -> Result<bool> {
        eprintln!("ScoutPack semantic search uses local `{DEFAULT_MODEL}` embeddings.");
        eprintln!(
            "This may download model files to {} on first use. Continue? [y/N]",
            cache_dir.display()
        );
        eprint!("> ");
        io::stderr().flush()?;

        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        Ok(matches!(answer.trim(), "y" | "Y" | "yes" | "YES"))
    }

    fn encode_vector(values: &[f32]) -> Vec<u8> {
        let mut out = Vec::with_capacity(values.len() * 4);
        for value in values {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out
    }

    fn decode_vector(blob: &[u8]) -> Vec<f32> {
        blob.chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
        let len = a.len().min(b.len());
        if len == 0 {
            return 0.0;
        }
        let mut dot = 0.0f64;
        let mut a_norm = 0.0f64;
        let mut b_norm = 0.0f64;
        for idx in 0..len {
            let av = a[idx] as f64;
            let bv = b[idx] as f64;
            dot += av * bv;
            a_norm += av * av;
            b_norm += bv * bv;
        }
        if a_norm == 0.0 || b_norm == 0.0 {
            0.0
        } else {
            dot / (a_norm.sqrt() * b_norm.sqrt())
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
}

#[cfg(test)]
mod tests {
    use super::hybrid_score;

    #[test]
    fn hybrid_score_weights_keyword_and_semantic_scores() {
        let keyword_heavy = hybrid_score(4.0, 0.25, 0.25);
        let semantic_heavy = hybrid_score(4.0, 0.25, 0.75);
        assert!(keyword_heavy > semantic_heavy);
        assert!((hybrid_score(0.0, 0.8, 1.0) - 0.8).abs() < f64::EPSILON);
        assert!((hybrid_score(4.0, 0.8, 0.0) - 1.0).abs() < f64::EPSILON);
    }
}
