# Design

ScoutPack turns a local repo into compact context packets.

Pipeline:

1. Walk repo with `.gitignore` and `.scoutpackignore`.
2. Require eligible regular files inside the canonical root before comparing size and nanosecond modification time with the local metadata cache.
3. Read, validate, and hash only new or metadata-changed files.
4. Skip unsupported, binary, large, and sensitive files.
5. Chunk changed files into meaningful units.
6. Update only affected files, chunks, symbols, imports, commands, edges, and FTS rows in SQLite.
7. Query SQLite FTS5.
8. Render the complete Markdown packet under ScoutPack's estimated token budget. If required task/revision metadata and section summaries cannot fit, return an actionable error. JSON/XML wrappers and template framing are outside this budget.

Git-scoped context resolves endpoints once and materializes eligible regular blobs from the content commit into a temporary index. Snippets, symbols, imports, and commands therefore share one revision; dirty working-tree files do not enter the packet. `--branch` uses the merge base with main, while explicit `--diff` compares the supplied endpoint trees. Temporary indexes are deleted after use and rebuilt on each request; this favors correctness over warm-cache latency. Semantic search is unavailable for these snapshots. See [Git-aware context](../examples/git-aware-context.md).

`search`, `context`, `template`, and index-backed MCP tools run this incremental refresh automatically. `--no-refresh` provides explicit frozen-index behavior for CLI queries. `watch` uses the same pipeline after its filesystem debounce.

`scoutpack doctor` checks database/schema/FTS health and runs a non-mutating freshness scan. `doctor --fix` invokes the same incremental pack path. ScoutPack may update files under `.scoutpack/`, but never edits source files or executes project commands.

Metadata reuse assumes ordinary filesystem behavior: content changes update size or modification time. A deliberate same-size edit with a restored identical modification timestamp requires an explicit metadata change or index rebuild.

Default installs avoid embeddings and command execution.

Optional semantic mode is gated behind the `semantic` Cargo feature. It can store local fastembed vectors in SQLite and combine FTS score with cosine similarity when users pass `--semantic`.

## Replay And File Boundaries

Default keyword retrieval orders FTS candidates by score and stable path/range/symbol/chunk fields before limiting candidates. Ranked search, import expansion, call expansion, and packet merging share score/path/range tie-breakers. Import reasons are sorted and deduplicated; call edges, commands, framework samples, and scan records have explicit stable ordering. Index row IDs and hash-map iteration do not define packet order.

Regression tests compare exact CLI payloads across separate processes, rebuilt indexes, compact/full packets, call expansion, equal-score searches, and repeated committed snapshots. The tested contract requires unchanged indexed content, configuration, query, budget, options, and executable version. It excludes stats/manifest timestamps and optional semantic inference. It does not promise byte identity across SQLite, grammar, or ScoutPack upgrades.

The scanner canonicalizes its root, requires a directory, disables directory-link traversal, rejects file symlinks and special files before metadata cache reuse, and checks canonical source containment. Unix fixtures cover external/internal/broken links, linked directories, sockets, root aliases, and regular files replaced by symlinks after indexing. Refresh removes previously indexed content when a source becomes ineligible; `--no-refresh` deliberately retains old indexed data.

These are source-scanner checks, not a sandbox. Metadata checks and file reads are separate operations: a hostile process can replace paths between them. Hard links and secret contents under ordinary source names are not detected as secrets. ScoutPack configuration and index-storage paths need their own trust review; do not run against a checkout concurrently modified by an adversary. Atomic no-follow reads and storage-path hardening remain separate work.
