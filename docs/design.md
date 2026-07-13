# Design

ScoutPack turns a local repo into compact context packets.

Pipeline:

1. Walk repo with `.gitignore` and `.scoutpackignore`.
2. Compare file size and nanosecond modification time with the local metadata cache.
3. Read, validate, and hash only new or metadata-changed files.
4. Skip unsupported, binary, large, and sensitive files.
5. Chunk changed files into meaningful units.
6. Update only affected files, chunks, symbols, imports, commands, edges, and FTS rows in SQLite.
7. Query SQLite FTS5.
8. Render markdown under approximate token budget.

`search`, `context`, `template`, and index-backed MCP tools run this incremental refresh automatically. `--no-refresh` provides explicit frozen-index behavior for CLI queries. `watch` uses the same pipeline after its filesystem debounce.

`scoutpack doctor` checks database/schema/FTS health and runs a non-mutating freshness scan. `doctor --fix` invokes the same incremental pack path. ScoutPack may update files under `.scoutpack/`, but never edits source files or executes project commands.

Metadata reuse assumes ordinary filesystem behavior: content changes update size or modification time. A deliberate same-size edit with a restored identical modification timestamp requires an explicit metadata change or index rebuild.

Default installs avoid embeddings and command execution.

Optional semantic mode is gated behind the `semantic` Cargo feature. It can store local fastembed vectors in SQLite and combine FTS score with cosine similarity when users pass `--semantic`.
