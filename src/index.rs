use crate::{chunk, config, git, scanner};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const INDEX_DIR: &str = ".scoutpack";
const DB_FILE: &str = "pack.sqlite";
const MANIFEST_FILE: &str = "manifest.json";
const REPO_MAP_FILE: &str = "repo-map.md";
const SCHEMA_VERSION: i64 = 1;

#[derive(Debug)]
pub struct PackSummary {
    pub files_indexed: usize,
    pub chunks_indexed: usize,
    pub files_skipped: usize,
    pub index_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub scoutpack_version: String,
    pub schema_version: i64,
    pub indexed_root_path: String,
    pub indexed_at: i64,
    pub config_hash: String,
    pub file_count: usize,
    pub skipped_count: usize,
    pub current_git_branch: Option<String>,
    pub recent_changed_files: Vec<String>,
    pub recent_commit_subjects: Vec<String>,
}

#[derive(Debug)]
pub struct IndexStats {
    pub manifest: Option<Manifest>,
    pub file_count: i64,
    pub chunk_count: i64,
    pub symbol_count: i64,
    pub command_count: i64,
    pub skipped_count: i64,
    pub index_path: PathBuf,
}

pub fn index_path(root: &Path) -> PathBuf {
    root.join(INDEX_DIR).join(DB_FILE)
}

pub fn ensure_index(root: &Path) -> Result<Connection> {
    let path = index_path(root);
    if !path.exists() {
        anyhow::bail!("No ScoutPack index found. Run `scoutpack pack .` first.");
    }
    Connection::open(&path).with_context(|| format!("SQLite index unreadable: {}", path.display()))
}

pub fn pack_repo(root: &Path) -> Result<PackSummary> {
    if !root.exists() {
        anyhow::bail!("Path does not exist: {}", root.display());
    }
    let root = root.canonicalize()?;
    let config = config::load(&root)?;
    let scan = scanner::scan_repo(&root, &config)?;

    let scout_dir = root.join(INDEX_DIR);
    fs::create_dir_all(&scout_dir)?;
    let db_path = scout_dir.join(DB_FILE);
    let mut conn = Connection::open(&db_path)
        .with_context(|| format!("Could not open SQLite index {}", db_path.display()))?;
    recreate_schema(&conn)?;

    let indexed_at = now_unix();
    let tx = conn.transaction()?;
    let mut chunks_indexed = 0usize;

    for file in &scan.files {
        tx.execute(
            "INSERT INTO files (path, language, kind, size_bytes, hash, mtime, indexed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                file.rel_path,
                file.language,
                file.kind,
                file.size_bytes as i64,
                file.hash,
                file.mtime,
                indexed_at
            ],
        )?;
        let file_id = tx.last_insert_rowid();
        let chunked = chunk::chunk_file(&file.rel_path, &file.language, &file.text);

        for chunk in chunked.chunks {
            let chunk_kind = chunk.kind;
            let chunk_name = chunk.name;
            let chunk_text = chunk.text;
            tx.execute(
                "INSERT INTO chunks (file_id, kind, name, start_line, end_line, text)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    file_id,
                    &chunk_kind,
                    &chunk_name,
                    chunk.start_line as i64,
                    chunk.end_line as i64,
                    &chunk_text
                ],
            )?;
            let chunk_id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO chunks_fts (rowid, path, kind, name, text)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    chunk_id,
                    &file.rel_path,
                    &chunk_kind,
                    &chunk_name.clone().unwrap_or_default(),
                    &chunk_text
                ],
            )?;
            chunks_indexed += 1;
        }

        for symbol in chunked.symbols {
            tx.execute(
                "INSERT INTO symbols (file_id, name, kind, start_line, end_line)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    file_id,
                    symbol.name,
                    symbol.kind,
                    symbol.start_line as i64,
                    symbol.end_line as i64
                ],
            )?;
        }

        for import in chunked.imports {
            tx.execute(
                "INSERT INTO imports (from_file_id, to_path, symbol)
                 VALUES (?1, ?2, ?3)",
                params![file_id, import.to_path, import.symbol],
            )?;
        }

        for command in chunked.commands {
            tx.execute(
                "INSERT INTO commands (name, command, source)
                 VALUES (?1, ?2, ?3)",
                params![command.name, command.command, command.source],
            )?;
        }
    }

    for skipped in &scan.skipped {
        tx.execute(
            "INSERT INTO skipped_files (path, reason) VALUES (?1, ?2)",
            params![skipped.path, skipped.reason],
        )?;
    }
    tx.commit()?;

    let manifest = Manifest {
        scoutpack_version: env!("CARGO_PKG_VERSION").to_owned(),
        schema_version: SCHEMA_VERSION,
        indexed_root_path: root.display().to_string(),
        indexed_at,
        config_hash: config::config_hash(&config)?,
        file_count: scan.files.len(),
        skipped_count: scan.skipped.len(),
        current_git_branch: git::current_branch(&root),
        recent_changed_files: git::recent_changed_files(&root),
        recent_commit_subjects: git::recent_commit_subjects(&root),
    };
    fs::write(
        scout_dir.join(MANIFEST_FILE),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    fs::write(
        scout_dir.join(REPO_MAP_FILE),
        render_repo_map(&scan.files, &scan.skipped),
    )?;

    Ok(PackSummary {
        files_indexed: scan.files.len(),
        chunks_indexed,
        files_skipped: scan.skipped.len(),
        index_path: db_path,
    })
}

pub fn read_stats(root: &Path) -> Result<IndexStats> {
    let conn = ensure_index(root)?;
    let path = index_path(root);
    let manifest = fs::read_to_string(root.join(INDEX_DIR).join(MANIFEST_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok());
    Ok(IndexStats {
        manifest,
        file_count: count(&conn, "files")?,
        chunk_count: count(&conn, "chunks")?,
        symbol_count: count(&conn, "symbols")?,
        command_count: count(&conn, "commands")?,
        skipped_count: count(&conn, "skipped_files")?,
        index_path: path,
    })
}

pub fn read_commands(conn: &Connection) -> Result<Vec<(String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT name, command, source
         FROM commands
         ORDER BY CASE name
           WHEN 'test' THEN 0
           WHEN 'lint' THEN 1
           WHEN 'typecheck' THEN 2
           WHEN 'build' THEN 3
           ELSE 4
         END, name",
    )?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn framework_signals(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT text FROM chunks WHERE kind = 'package-dependencies' LIMIT 5")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut signals = Vec::new();
    for row in rows {
        let text = row?;
        for (needle, name) in [
            ("\"next\"", "Next.js"),
            ("\"react\"", "React"),
            ("\"vite\"", "Vite"),
            ("\"vitest\"", "Vitest"),
            ("\"jest\"", "Jest"),
            ("\"playwright\"", "Playwright"),
        ] {
            if text.contains(needle) && !signals.iter().any(|signal| signal == name) {
                signals.push(name.to_owned());
            }
        }
    }
    Ok(signals)
}

fn recreate_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        PRAGMA foreign_keys = OFF;
        DROP TABLE IF EXISTS chunks_fts;
        DROP TABLE IF EXISTS imports;
        DROP TABLE IF EXISTS symbols;
        DROP TABLE IF EXISTS chunks;
        DROP TABLE IF EXISTS commands;
        DROP TABLE IF EXISTS skipped_files;
        DROP TABLE IF EXISTS files;
        PRAGMA foreign_keys = ON;

        CREATE TABLE files (
          id INTEGER PRIMARY KEY,
          path TEXT NOT NULL UNIQUE,
          language TEXT NOT NULL,
          kind TEXT NOT NULL,
          size_bytes INTEGER NOT NULL,
          hash TEXT NOT NULL,
          mtime INTEGER NOT NULL,
          indexed_at INTEGER NOT NULL
        );

        CREATE TABLE chunks (
          id INTEGER PRIMARY KEY,
          file_id INTEGER NOT NULL,
          kind TEXT NOT NULL,
          name TEXT,
          start_line INTEGER NOT NULL,
          end_line INTEGER NOT NULL,
          text TEXT NOT NULL,
          FOREIGN KEY(file_id) REFERENCES files(id)
        );

        CREATE TABLE symbols (
          id INTEGER PRIMARY KEY,
          file_id INTEGER NOT NULL,
          name TEXT NOT NULL,
          kind TEXT NOT NULL,
          start_line INTEGER NOT NULL,
          end_line INTEGER NOT NULL,
          FOREIGN KEY(file_id) REFERENCES files(id)
        );

        CREATE TABLE imports (
          id INTEGER PRIMARY KEY,
          from_file_id INTEGER NOT NULL,
          to_path TEXT NOT NULL,
          symbol TEXT,
          FOREIGN KEY(from_file_id) REFERENCES files(id)
        );

        CREATE TABLE commands (
          id INTEGER PRIMARY KEY,
          name TEXT NOT NULL,
          command TEXT NOT NULL,
          source TEXT NOT NULL
        );

        CREATE TABLE skipped_files (
          id INTEGER PRIMARY KEY,
          path TEXT NOT NULL,
          reason TEXT NOT NULL
        );

        CREATE VIRTUAL TABLE chunks_fts USING fts5(
          path,
          kind,
          name,
          text,
          content='',
          tokenize='unicode61'
        );
        ",
    )?;
    Ok(())
}

fn count(conn: &Connection, table: &str) -> Result<i64> {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    conn.query_row(&sql, [], |row| row.get(0))
        .map_err(Into::into)
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn render_repo_map(files: &[scanner::ScannedFile], skipped: &[scanner::SkippedFile]) -> String {
    let mut out = String::from("# ScoutPack Repo Map\n\n## Indexed Files\n");
    for file in files {
        out.push_str(&format!(
            "- `{}` [{}:{}] ({} bytes)\n",
            file.rel_path, file.language, file.kind, file.size_bytes
        ));
    }
    out.push_str("\n## Skipped Files\n");
    for skipped in skipped {
        out.push_str(&format!("- `{}`: {}\n", skipped.path, skipped.reason));
    }
    out
}
