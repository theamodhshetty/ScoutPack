# Milestones

ScoutPack development stays milestone-driven. Each major milestone should end with:

- focused commit
- passing `cargo fmt --check`
- passing `cargo test`
- README or docs update when user-facing behavior changes

## Completed

### M1: OSS CLI Scaffold

- Rust crate
- `clap` CLI commands
- README, license, roadmap, security, contribution docs
- CI workflow

### M2: Init And Config

- `scoutpack init`
- default `scoutpack.toml`
- default `.scoutpackignore`
- safe repeated init

### M3: Scanner And SQLite Index

- repo walker with `.gitignore` and `.scoutpackignore`
- sensitive-file skip rules
- size and binary skip rules
- SQLite schema
- manifest and repo map output

### M4: Search And Context

- Markdown, package.json, config, and TS/TSX chunking
- SQLite FTS5 search
- token-budgeted context packet renderer
- package script and framework signals
- fixture integration test

## Next

### M5: Incremental Indexing

- reuse unchanged files on repeated `pack`
- only re-chunk changed files
- track indexed, reused, removed, and skipped counts

### M6: Parser Quality

- tree-sitter TypeScript and TSX symbol extraction
- imports with source symbols
- route and component detection with fewer false positives

### M7: Context Quality

- better task-term ranking
- import proximity boost
- risk hints grounded by indexed evidence
- tighter token budgeting

### M8: Public Release Prep

- GitHub repo topics and description
- release checklist
- crates.io metadata cleanup
- installation docs

### M9: MCP v0.2

- read-only MCP server
- `search`, `context`, `file_summary`, `symbol`, `commands`, `stats`
- no write, network, or shell execution capabilities

