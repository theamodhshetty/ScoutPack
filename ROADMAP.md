# Roadmap

## v0.1

- Rust CLI with `init`, `pack`, `search`, `context`, and `stats`.
- Local SQLite index with FTS5.
- TypeScript, TSX, Markdown, JSON, YAML, and TOML support.
- package.json script and framework detection.
- Token-budgeted markdown context packets.
- No cloud calls, telemetry, auto-edits, or project command execution.
- Incremental indexing for unchanged files.
- tree-sitter TypeScript/TSX symbol extraction.
- context ranking that favors likely edit files.
- release-ready docs and examples.

## v0.2

- Read-only MCP server.
- MCP tools: `search`, `context`, `file_summary`, `symbol`, `commands`, `stats`.
- Client config examples for popular AI coding tools.

## Later

- Local vector search only after deterministic ranking works.
- crates.io, Homebrew, and optional npm wrapper distribution.

See [docs/milestones.md](docs/milestones.md) for milestone tracking.
