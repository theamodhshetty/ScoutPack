# Changelog

All notable ScoutPack changes are tracked here.

## Unreleased

### Added

- Benchmark methodology docs and `scripts/bench-one-repo.sh` for local repo measurement.
- Positioning doc and launch-focused roadmap/milestone reset around ScoutPack as a context preflight layer.
- Demo and comparison docs plus terminal-style README demo asset.
- `WHY.md` and risk-hint documentation.
- JavaScript and JSX symbol/import extraction, including ESM imports, CommonJS requires, components, and Express-style route handlers.
- Python and Go config expansion for `pyproject.toml`, `requirements.txt`, `Pipfile`, `setup.cfg`, and `go.mod` commands/framework signals.
- Git-aware context flags: `--since`, `--diff`, and `--branch`, plus MCP `recent_changes`.
- Criterion benchmark coverage and real-repo benchmark script/results for cold indexing, incremental indexing, packet reduction, and search latency.
- `scoutpack watch` mode for debounced local index refreshes during active development.
- Optional local semantic search behind the `semantic` Cargo feature, with `pack --embed`, `search --semantic`, and `context --semantic`.
- Release binary workflow, Unix install script, Homebrew formula template, and Scoop manifest template.
- Prompt template library with `scoutpack template`, custom Markdown templates, and MCP `template` tool.
- HTTP/SSE MCP mode via `scoutpack mcp . --http --port 7777`.
- Basic symbol call graph indexing and `context --expand-calls` expansion.
- Machine-readable JSON output for `search`, `context`, and `stats`.
- Context output formats: markdown, JSON, and XML.
- Python and Rust symbol/import extraction.
- Go and Solidity symbol/import extraction.
- Read-only MCP server with `search`, `context`, `file_summary`, `symbol`, `commands`, and `stats` tools.
- Shell completion generation for bash, zsh, fish, PowerShell, and Elvish.
- Project logo and redesigned README landing page.
- Efficiency model graphic and agent client setup docs for MCP, Codex, Claude Code, GitHub Copilot, Cursor, and Aider.
- Discoverability docs for AI agent workflows, use cases, and FAQ.
- GitHub issue templates and pull request template.

### Changed

- Renamed context packet `Risks` section to `Risk Hints` and added confidence labels to avoid implying static-analysis proof.

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

- Expanded sensitive-file skip tests and generated-folder skip coverage.
- Sensitive file patterns are skipped by default.
- No telemetry, cloud calls, API keys, auto-edits, or project command execution.
