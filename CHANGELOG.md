# Changelog

All notable ScoutPack changes are tracked here.

## Unreleased

### Added

- MCP `context` Git scopes (`since`, `diff`, `branch`) for commit-pinned review packets, with schema discovery and dirty-tree regression coverage.

- `scoutpack doctor` health and freshness diagnostics with text, JSON, and `--fix` modes.
- Automatic first-use and stale-index refresh for `search`, `context`, `template`, and index-backed MCP tools, with `--no-refresh` for frozen CLI queries.
- Pinned real-repo retrieval benchmarks with Top-1/3/5 hits and expected-file coverage.
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

- Git-scoped MCP context/template requests avoid working-tree refresh; invalid or conflicting scopes fail before indexing. MCP rejects malformed and three-dot diff ranges.

- Reorganized README around installation, task-based workflows, local MCP search, and concise language/privacy support. Added documentation index and first-run troubleshooting; clarified unpublished binary installation and removed keyword stuffing.

- Git-scoped context now reads a temporary index of the resolved content commit, includes full base/content commit IDs, and excludes staged, unstaged, and untracked content. Branch mode uses the merge base with main; explicit diff keeps two-endpoint semantics. Git-scoped semantic search reports an actionable unsupported-combination error.

- Reordered active roadmap and milestones around whole-packet budget correctness, review evidence, verified installation, and measured external use; retained previous milestone planning as superseded history.

- Incremental indexing now avoids reading and hashing metadata-unchanged files and updates only affected FTS rows instead of rebuilding the entire FTS table.
- Benchmark scripts now isolate retrieval/render latency with `--no-refresh` and parse chunk counts independently from embedding counts.
- Benchmark reports now separate packet compression from retrieval quality and include host/toolchain metadata.
- Source ranking boost now applies consistently to Python, Rust, Go, and Solidity, not only JavaScript/TypeScript.
- Task query normalization removes generic edit verbs and maps common coding nouns such as `registration` to symbol forms such as `register`.
- Context snippet admission now honors requested token budget instead of allowing 10% overhead.
- Renamed context packet `Risks` section to `Risk Hints` and added confidence labels to avoid implying static-analysis proof.

### Fixed

- Source scanner no longer follows file symlinks into external content or reuses cached entries for non-regular files. Refresh removes source content replaced by symlinks.
- Default keyword retrieval and packets now use stable tie-breakers before candidate limits and ranked truncation, with ordered import reasons, call edges, commands, framework samples, and scan records.

- Enforced the estimator budget across the complete context packet, including fallback metadata, commands, and hints. Too-small budgets fail clearly instead of returning oversized packets; JSON/XML wrappers and template framing remain outside the packet budget.

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
