use crate::{chunk, config, git, scanner, semantic};
use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const INDEX_DIR: &str = ".scoutpack";
const DB_FILE: &str = "pack.sqlite";
const MANIFEST_FILE: &str = "manifest.json";
const REPO_MAP_FILE: &str = "repo-map.md";
pub const SCHEMA_VERSION: i64 = 5;

#[derive(Debug, Clone, Copy, Default)]
pub struct PackOptions {
    pub embed: bool,
    pub allow_model_download: bool,
}

#[derive(Debug, Serialize)]
pub struct PackSummary {
    pub files_indexed: usize,
    pub files_reused: usize,
    pub files_removed: usize,
    pub chunks_indexed: usize,
    pub embeddings_indexed: usize,
    pub files_skipped: usize,
    pub files_read: usize,
    pub metadata_reused: usize,
    pub index_path: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct IndexFreshness {
    pub changed_files: usize,
    pub removed_files: usize,
    pub config_changed: bool,
    pub files_read: usize,
    pub metadata_reused: usize,
}

#[derive(Debug, Serialize)]
pub struct LanguageCount {
    pub language: String,
    pub files: i64,
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
    #[serde(default)]
    pub git_head: Option<String>,
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
    pub embedding_count: i64,
    pub skipped_count: i64,
    pub index_path: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct IndexedCommand {
    pub name: String,
    pub command: String,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct FileChunkSummary {
    pub kind: String,
    pub name: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Serialize)]
pub struct IndexedSymbol {
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Serialize)]
pub struct FileSummary {
    pub path: String,
    pub language: String,
    pub kind: String,
    pub size_bytes: usize,
    pub indexed_at: i64,
    pub symbols: Vec<IndexedSymbol>,
    pub chunks: Vec<FileChunkSummary>,
}

#[derive(Debug, Serialize)]
pub struct SymbolMatch {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
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
    pack_repo_with_options(root, PackOptions::default())
}

pub fn pack_repo_with_options(root: &Path, options: PackOptions) -> Result<PackSummary> {
    if !root.exists() {
        anyhow::bail!("Path does not exist: {}", root.display());
    }
    let root = root.canonicalize()?;
    let scout_dir = root.join(INDEX_DIR);
    fs::create_dir_all(&scout_dir)?;
    let db_path = scout_dir.join(DB_FILE);
    let mut conn = Connection::open(&db_path)
        .with_context(|| format!("Could not open SQLite index {}", db_path.display()))?;
    ensure_schema(&conn)?;

    let existing = existing_files(&conn)?;
    let cache = existing
        .iter()
        .map(|(path, file)| {
            (
                path.clone(),
                scanner::CachedFileMetadata {
                    language: file.language.clone(),
                    kind: file.kind.clone(),
                    size_bytes: file.size_bytes,
                    hash: file.hash.clone(),
                    mtime_ns: file.mtime_ns,
                },
            )
        })
        .collect();
    let config = config::load(&root)?;
    let scan = scanner::scan_repo_with_cache(&root, &config, &cache)?;
    let existing_skipped = existing_skipped_files(&conn)?;
    let scanned_skipped: HashSet<(String, String)> = scan
        .skipped
        .iter()
        .map(|file| (file.path.clone(), file.reason.clone()))
        .collect();
    let skipped_changed = existing_skipped != scanned_skipped;
    let scan_paths: HashSet<&str> = scan
        .files
        .iter()
        .map(|file| file.rel_path.as_str())
        .collect();
    let files_removed = existing
        .keys()
        .filter(|path| !scan_paths.contains(path.as_str()))
        .count();
    let scanned_by_path: HashMap<&str, &scanner::ScannedFile> = scan
        .files
        .iter()
        .map(|file| (file.rel_path.as_str(), file))
        .collect();

    let indexed_at = now_unix();
    let tx = conn.transaction()?;
    if skipped_changed {
        tx.execute("DELETE FROM skipped_files", [])?;
    }

    let mut chunks_indexed = 0usize;
    let mut files_indexed = 0usize;
    let mut files_reused = 0usize;

    for (path, existing_file) in &existing {
        let changed = scanned_by_path
            .get(path.as_str())
            .is_some_and(|file| file.hash != existing_file.hash);
        let removed = !scan_paths.contains(path.as_str());
        if changed || removed {
            delete_file(&tx, existing_file.id, path)?;
        }
    }

    for file in &scan.files {
        if let Some(existing_file) = existing.get(&file.rel_path) {
            if existing_file.hash == file.hash {
                if existing_file.language != file.language
                    || existing_file.kind != file.kind
                    || existing_file.size_bytes != file.size_bytes
                    || existing_file.mtime_ns != file.mtime_ns
                {
                    tx.execute(
                        "UPDATE files
                         SET language = ?1, kind = ?2, size_bytes = ?3, mtime_ns = ?4
                         WHERE id = ?5",
                        params![
                            file.language,
                            file.kind,
                            file.size_bytes as i64,
                            file.mtime_ns,
                            existing_file.id
                        ],
                    )?;
                }
                files_reused += 1;
                continue;
            }
        }

        chunks_indexed += insert_scanned_file(&tx, file, indexed_at)?;
        files_indexed += 1;
    }

    if skipped_changed {
        for skipped in &scan.skipped {
            tx.execute(
                "INSERT INTO skipped_files (path, reason) VALUES (?1, ?2)",
                params![skipped.path, skipped.reason],
            )?;
        }
    }
    tx.commit()?;
    let embeddings_indexed = if options.embed {
        semantic::embed_missing_chunks(&conn, options.allow_model_download)?
    } else {
        0
    };

    let manifest = Manifest {
        scoutpack_version: env!("CARGO_PKG_VERSION").to_owned(),
        schema_version: SCHEMA_VERSION,
        indexed_root_path: root.display().to_string(),
        indexed_at,
        config_hash: config::config_hash(&config)?,
        file_count: scan.files.len(),
        skipped_count: scan.skipped.len(),
        current_git_branch: git::current_branch(&root),
        git_head: git::head_commit(&root),
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
        files_indexed,
        files_reused,
        files_removed,
        chunks_indexed,
        embeddings_indexed,
        files_skipped: scan.skipped.len(),
        files_read: scan.files_read,
        metadata_reused: scan.metadata_reused,
        index_path: db_path,
    })
}

pub fn read_stats(root: &Path) -> Result<IndexStats> {
    let conn = ensure_index(root)?;
    let path = index_path(root);
    let manifest = read_manifest(root);
    Ok(IndexStats {
        manifest,
        file_count: count(&conn, "files")?,
        chunk_count: count(&conn, "chunks")?,
        symbol_count: count(&conn, "symbols")?,
        command_count: count(&conn, "commands")?,
        embedding_count: count(&conn, "chunk_embeddings")?,
        skipped_count: count(&conn, "skipped_files")?,
        index_path: path,
    })
}

pub fn read_manifest(root: &Path) -> Option<Manifest> {
    fs::read_to_string(root.join(INDEX_DIR).join(MANIFEST_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
}

pub fn database_schema_version(root: &Path) -> Result<Option<i64>> {
    let path = index_path(root);
    if !path.exists() {
        return Ok(None);
    }
    let conn = Connection::open(&path)
        .with_context(|| format!("SQLite index unreadable: {}", path.display()))?;
    schema_version(&conn)
}

pub fn fts_available(conn: &Connection) -> Result<bool> {
    table_exists(conn, "chunks_fts")
}

pub fn language_breakdown(conn: &Connection) -> Result<Vec<LanguageCount>> {
    let mut stmt = conn.prepare(
        "SELECT language, COUNT(*)
         FROM files
         GROUP BY language
         ORDER BY COUNT(*) DESC, language",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(LanguageCount {
            language: row.get(0)?,
            files: row.get(1)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn inspect_freshness(root: &Path) -> Result<IndexFreshness> {
    let root = root.canonicalize()?;
    let conn = ensure_index(&root)?;
    let existing = existing_files(&conn)?;
    let cache = existing
        .iter()
        .map(|(path, file)| {
            (
                path.clone(),
                scanner::CachedFileMetadata {
                    language: file.language.clone(),
                    kind: file.kind.clone(),
                    size_bytes: file.size_bytes,
                    hash: file.hash.clone(),
                    mtime_ns: file.mtime_ns,
                },
            )
        })
        .collect();
    let config = config::load(&root)?;
    let scan = scanner::scan_repo_with_cache(&root, &config, &cache)?;
    let scanned: HashMap<&str, &scanner::ScannedFile> = scan
        .files
        .iter()
        .map(|file| (file.rel_path.as_str(), file))
        .collect();
    let changed_files = scan
        .files
        .iter()
        .filter(|file| {
            existing
                .get(&file.rel_path)
                .is_none_or(|cached| cached.hash != file.hash)
        })
        .count();
    let removed_files = existing
        .keys()
        .filter(|path| !scanned.contains_key(path.as_str()))
        .count();
    let config_changed = read_manifest(&root).is_none_or(|manifest| {
        config::config_hash(&config).ok().as_ref() != Some(&manifest.config_hash)
    });

    Ok(IndexFreshness {
        changed_files,
        removed_files,
        config_changed,
        files_read: scan.files_read,
        metadata_reused: scan.metadata_reused,
    })
}

pub fn read_commands(conn: &Connection) -> Result<Vec<(String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT name, command, MIN(source) AS source
         FROM commands
         GROUP BY name, command
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

pub fn read_indexed_commands(conn: &Connection) -> Result<Vec<IndexedCommand>> {
    Ok(read_commands(conn)?
        .into_iter()
        .map(|(name, command, source)| IndexedCommand {
            name,
            command,
            source,
        })
        .collect())
}

pub fn read_file_summary(conn: &Connection, path: &str) -> Result<Option<FileSummary>> {
    let path = path.strip_prefix("./").unwrap_or(path);
    let file = conn
        .query_row(
            "SELECT id, path, language, kind, size_bytes, indexed_at
             FROM files
             WHERE path = ?1",
            params![path],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()?;

    let Some((file_id, path, language, kind, size_bytes, indexed_at)) = file else {
        return Ok(None);
    };

    let mut symbol_stmt = conn.prepare(
        "SELECT name, kind, start_line, end_line
         FROM symbols
         WHERE file_id = ?1
         ORDER BY start_line, name",
    )?;
    let symbol_rows = symbol_stmt.query_map(params![file_id], |row| {
        Ok(IndexedSymbol {
            name: row.get(0)?,
            kind: row.get(1)?,
            start_line: row.get::<_, i64>(2)? as usize,
            end_line: row.get::<_, i64>(3)? as usize,
        })
    })?;
    let symbols = symbol_rows.collect::<rusqlite::Result<Vec<_>>>()?;

    let mut chunk_stmt = conn.prepare(
        "SELECT kind, name, start_line, end_line
         FROM chunks
         WHERE file_id = ?1
         ORDER BY start_line
         LIMIT 50",
    )?;
    let chunk_rows = chunk_stmt.query_map(params![file_id], |row| {
        Ok(FileChunkSummary {
            kind: row.get(0)?,
            name: row.get(1)?,
            start_line: row.get::<_, i64>(2)? as usize,
            end_line: row.get::<_, i64>(3)? as usize,
        })
    })?;
    let chunks = chunk_rows.collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(Some(FileSummary {
        path,
        language,
        kind,
        size_bytes: size_bytes as usize,
        indexed_at,
        symbols,
        chunks,
    }))
}

pub fn find_symbols(conn: &Connection, query: &str, limit: usize) -> Result<Vec<SymbolMatch>> {
    if query.trim().is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let like = format!("%{}%", query.trim().to_ascii_lowercase());
    let mut stmt = conn.prepare(
        "SELECT f.path, s.name, s.kind, s.start_line, s.end_line
         FROM symbols s
         JOIN files f ON s.file_id = f.id
         WHERE lower(s.name) LIKE ?1
         ORDER BY
           CASE WHEN lower(s.name) = ?2 THEN 0 ELSE 1 END,
           f.path,
           s.start_line
         LIMIT ?3",
    )?;
    let rows = stmt.query_map(
        params![like, query.trim().to_ascii_lowercase(), limit as i64],
        |row| {
            Ok(SymbolMatch {
                path: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                start_line: row.get::<_, i64>(3)? as usize,
                end_line: row.get::<_, i64>(4)? as usize,
            })
        },
    )?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn framework_signals(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT text FROM chunks
         WHERE kind IN (
           'package-dependencies',
           'python-dependencies',
           'python-config',
           'go-module',
           'go-dependencies'
         )
         LIMIT 25",
    )?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut signals = Vec::new();
    for row in rows {
        let text = row?.to_ascii_lowercase();
        for (needle, name) in [
            ("\"next\"", "Next.js"),
            ("\"react\"", "React"),
            ("\"vite\"", "Vite"),
            ("\"vitest\"", "Vitest"),
            ("\"jest\"", "Jest"),
            ("\"playwright\"", "Playwright"),
            ("fastapi", "FastAPI"),
            ("django", "Django"),
            ("flask", "Flask"),
            ("pydantic", "Pydantic"),
            ("pytest", "Pytest"),
            ("language: go", "Go"),
            ("github.com/gin-gonic/gin", "Gin"),
            ("github.com/labstack/echo", "Echo"),
            ("github.com/gofiber/fiber", "Fiber"),
            ("github.com/go-chi/chi", "Chi"),
            ("google.golang.org/grpc", "gRPC"),
        ] {
            if text.contains(needle) && !signals.iter().any(|signal| signal == name) {
                signals.push(name.to_owned());
            }
        }
    }
    Ok(signals)
}

#[derive(Debug)]
struct ExistingFile {
    id: i64,
    language: String,
    kind: String,
    size_bytes: u64,
    hash: String,
    mtime_ns: i64,
}

fn existing_files(conn: &Connection) -> Result<HashMap<String, ExistingFile>> {
    let mut stmt =
        conn.prepare("SELECT id, path, language, kind, size_bytes, hash, mtime_ns FROM files")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(1)?,
            ExistingFile {
                id: row.get(0)?,
                language: row.get(2)?,
                kind: row.get(3)?,
                size_bytes: row.get::<_, i64>(4)? as u64,
                hash: row.get(5)?,
                mtime_ns: row.get(6)?,
            },
        ))
    })?;
    rows.collect::<rusqlite::Result<HashMap<_, _>>>()
        .map_err(Into::into)
}

fn existing_skipped_files(conn: &Connection) -> Result<HashSet<(String, String)>> {
    let mut stmt = conn.prepare("SELECT path, reason FROM skipped_files")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect::<rusqlite::Result<HashSet<_>>>()
        .map_err(Into::into)
}

fn insert_scanned_file(
    tx: &Transaction<'_>,
    file: &scanner::ScannedFile,
    indexed_at: i64,
) -> Result<usize> {
    tx.execute(
        "INSERT INTO files (path, language, kind, size_bytes, hash, mtime_ns, indexed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            file.rel_path,
            file.language,
            file.kind,
            file.size_bytes as i64,
            file.hash,
            file.mtime_ns,
            indexed_at
        ],
    )?;
    let file_id = tx.last_insert_rowid();
    let text = file
        .text
        .as_deref()
        .context("changed file content missing during indexing")?;
    let chunked = chunk::chunk_file(&file.rel_path, &file.language, text);
    let chunk_count = chunked.chunks.len();

    for chunk in chunked.chunks {
        tx.execute(
            "INSERT INTO chunks (file_id, kind, name, start_line, end_line, text)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                file_id,
                &chunk.kind,
                &chunk.name,
                chunk.start_line as i64,
                chunk.end_line as i64,
                &chunk.text
            ],
        )?;
        let chunk_id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO chunks_fts (rowid, path, kind, name, text)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                chunk_id,
                &file.rel_path,
                &chunk.kind,
                chunk.name.as_deref().unwrap_or(""),
                &chunk.text
            ],
        )?;
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

    for call in chunked.calls {
        tx.execute(
            "INSERT INTO symbol_edges (from_file_id, from_symbol, to_symbol, line)
             VALUES (?1, ?2, ?3, ?4)",
            params![file_id, call.from_symbol, call.to_symbol, call.line as i64],
        )?;
    }

    for command in chunked.commands {
        tx.execute(
            "INSERT INTO commands (name, command, source)
             VALUES (?1, ?2, ?3)",
            params![command.name, command.command, command.source],
        )?;
    }

    Ok(chunk_count)
}

fn delete_file(tx: &Transaction<'_>, file_id: i64, path: &str) -> Result<()> {
    tx.execute(
        "DELETE FROM chunks_fts WHERE rowid IN (SELECT id FROM chunks WHERE file_id = ?1)",
        params![file_id],
    )?;
    tx.execute(
        "DELETE FROM imports WHERE from_file_id = ?1",
        params![file_id],
    )?;
    tx.execute(
        "DELETE FROM symbol_edges WHERE from_file_id = ?1",
        params![file_id],
    )?;
    tx.execute("DELETE FROM symbols WHERE file_id = ?1", params![file_id])?;
    tx.execute(
        "DELETE FROM chunk_embeddings WHERE chunk_id IN (SELECT id FROM chunks WHERE file_id = ?1)",
        params![file_id],
    )?;
    tx.execute("DELETE FROM chunks WHERE file_id = ?1", params![file_id])?;
    tx.execute("DELETE FROM commands WHERE source = ?1", params![path])?;
    tx.execute("DELETE FROM files WHERE id = ?1", params![file_id])?;
    Ok(())
}

fn ensure_schema(conn: &Connection) -> Result<()> {
    if schema_version(conn)? != Some(SCHEMA_VERSION) || !schema_is_complete(conn)? {
        recreate_schema(conn)?;
    }
    Ok(())
}

fn schema_is_complete(conn: &Connection) -> Result<bool> {
    for table in [
        "meta",
        "files",
        "chunks",
        "chunk_embeddings",
        "symbols",
        "imports",
        "symbol_edges",
        "commands",
        "skipped_files",
        "chunks_fts",
    ] {
        if !table_exists(conn, table)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn schema_version(conn: &Connection) -> Result<Option<i64>> {
    if !table_exists(conn, "meta")? {
        return Ok(None);
    }
    let version = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .and_then(|value| value.parse::<i64>().ok());
    Ok(version)
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![table],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    Ok(exists)
}

fn recreate_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        PRAGMA foreign_keys = OFF;
        DROP TABLE IF EXISTS chunks_fts;
        DROP TABLE IF EXISTS imports;
        DROP TABLE IF EXISTS symbol_edges;
        DROP TABLE IF EXISTS symbols;
        DROP TABLE IF EXISTS chunk_embeddings;
        DROP TABLE IF EXISTS chunks;
        DROP TABLE IF EXISTS commands;
        DROP TABLE IF EXISTS skipped_files;
        DROP TABLE IF EXISTS files;
        DROP TABLE IF EXISTS meta;
        PRAGMA foreign_keys = ON;

        CREATE TABLE meta (
          key TEXT PRIMARY KEY,
          value TEXT NOT NULL
        );

        CREATE TABLE files (
          id INTEGER PRIMARY KEY,
          path TEXT NOT NULL UNIQUE,
          language TEXT NOT NULL,
          kind TEXT NOT NULL,
          size_bytes INTEGER NOT NULL,
          hash TEXT NOT NULL,
          mtime_ns INTEGER NOT NULL,
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

        CREATE TABLE chunk_embeddings (
          chunk_id INTEGER NOT NULL,
          model TEXT NOT NULL,
          dim INTEGER NOT NULL,
          vector BLOB NOT NULL,
          PRIMARY KEY (chunk_id, model),
          FOREIGN KEY(chunk_id) REFERENCES chunks(id)
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

        CREATE TABLE symbol_edges (
          id INTEGER PRIMARY KEY,
          from_file_id INTEGER NOT NULL,
          from_symbol TEXT NOT NULL,
          to_symbol TEXT NOT NULL,
          line INTEGER NOT NULL,
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
          tokenize='unicode61'
        );
        ",
    )?;
    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('schema_version', ?1)",
        params![SCHEMA_VERSION.to_string()],
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
