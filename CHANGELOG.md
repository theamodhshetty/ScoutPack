# Changelog

All notable ScoutPack changes are tracked here.

## Unreleased

### Added

- Machine-readable JSON output for `search`, `context`, and `stats`.
- Read-only MCP server with `search`, `context`, `file_summary`, `symbol`, `commands`, and `stats` tools.
- Discoverability docs for AI agent workflows, use cases, and FAQ.
- GitHub issue templates and pull request template.

## v0.1.0 - 2026-05-12

### Added

- Rust CLI with `init`, `pack`, `search`, `context`, and `stats`.
- Local SQLite index with FTS5 search.
- Incremental indexing for unchanged files.
- TypeScript and TSX parsing with tree-sitter.
- Markdown, JSON, YAML, and TOML scanning.
- package.json script and framework detection.
- Token-budgeted markdown context packets.
- Context ranking that favors likely edit source files over docs for coding tasks.
- Import proximity enrichment for related helper files.
- Grounded risk hints with source ranges.
- OSS docs, security policy, roadmap, and CI.

### Security

- Sensitive file patterns are skipped by default.
- No telemetry, cloud calls, API keys, auto-edits, or project command execution.
