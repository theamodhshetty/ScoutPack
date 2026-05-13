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

### M5: Incremental Indexing

- reuse unchanged files on repeated `pack`
- only re-chunk changed files
- track indexed, reused, removed, and skipped counts

### M6: Parser Quality

- tree-sitter TypeScript and TSX symbol extraction
- route handler detection
- exported type/interface and const component extraction
- heuristic parser fallback for parse errors

### M7: Context Quality

- preserve required Commands and Risks sections under tight budgets
- keep relevant files in ranking order
- improve task-term ranking
- add import proximity boost
- prefer likely edit source files over docs for coding tasks
- ground risk hints with indexed evidence
- cite source ranges for context packet risk hints

### M8: Public Release Prep

- GitHub repo topics and description
- release checklist
- crates.io metadata cleanup
- installation docs
- README demo output and support matrix

### M8.1: Machine-readable CLI Output

- `search --json`
- `context --json`
- `stats --json`

### M9: MCP v0.2

- read-only MCP server
- `search`, `context`, `file_summary`, `symbol`, `commands`, `stats`
- no write, network, or shell execution capabilities

### M10: Packaging And Distribution

- crates.io release checklist
- shell completion generation
- distribution docs

### M10.1: Project Page Polish

- SVG logo and README wordmark
- sharper README first screen
- MCP setup surfaced on main page
- discoverability search terms kept visible

### M10.2: Agent Adoption Docs

- efficiency model graphic
- MCP config examples for common clients
- agent workflow matrix
- AGENTS.md usage hint

### M10.3: Security And Context Format Foundation

- expanded sensitive-file skip coverage
- generated-folder skip coverage
- context output formats: markdown, JSON, XML
- token budget summary in context packets

## Next

### M11: Index Coverage

- JavaScript and JSX chunking and symbol extraction
- Python chunking and symbol extraction
- Rust symbol extraction
- Go chunking and symbol extraction
- Solidity chunking and symbol extraction
- lockfile/package manager detection
- Python and Go config command/framework extraction
- Git-aware context with `--since`, `--diff`, `--branch`, and MCP `recent_changes`
- Criterion benchmarks and reproducible real-repo benchmark results
- Watch mode with debounced incremental re-pack output
- Optional local semantic embeddings behind `--features semantic`
- Release binaries, install script, Homebrew formula template, and Scoop manifest template
- monorepo workspace summaries
