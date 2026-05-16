# Milestones

ScoutPack development stays milestone-driven. Each major milestone should end with:

- focused commit
- passing `cargo fmt --check`
- passing `cargo test`
- docs update when user-facing behavior changes
- changelog entry

## Strategy Shift

ScoutPack has enough core product surface for early adoption testing. Next milestones prioritize clarity, installation, demos, and ecosystem placement over adding more code intelligence features.

Product thesis:

> ScoutPack helps AI coding agents start with the right files, not the whole repo.

Execution thesis:

> Make one workflow obviously useful before expanding scope.

Primary workflow:

```bash
scoutpack context "review my PR" --branch --expand-calls 1 --budget 3000
```

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
- no write, outbound network, or shell execution capabilities

### M10: Packaging And Distribution Foundation

- crates.io release checklist
- shell completion generation
- distribution docs
- release binary workflow
- install script
- Homebrew formula template
- Scoop manifest template

### M11: Agent Context Core

- JavaScript and JSX chunking and symbol extraction
- Python chunking and symbol extraction
- Rust symbol extraction
- Go chunking and symbol extraction
- Solidity chunking and symbol extraction
- Python and Go config command/framework extraction
- Git-aware context with `--since`, `--diff`, `--branch`, and MCP `recent_changes`
- Criterion benchmarks and reproducible real-repo benchmark results
- Watch mode with debounced incremental re-pack output
- Optional local semantic embeddings behind `--features semantic`
- Prompt template library with built-in and custom Markdown templates
- HTTP/SSE MCP mode for clients that do not use stdio
- Basic symbol call graph with `context --expand-calls`

## Next

### M12: Positioning And Demo Readiness

Goal: make ScoutPack understandable in 30 seconds.

Status: in progress.

Done:

- README first screen around "context preflight"
- terminal-style README demo asset
- `docs/positioning.md`
- `docs/demo.md`
- `docs/comparison.md`
- "ScoutPack vs Repomix/Aider/Cursor/Sourcegraph" comparison
- real PR-review demo output

Remaining:

- terminal GIF or asciinema recording when recording tooling is available
- update repo description/topics if needed

Acceptance:

- README explains use case without reading roadmap
- demo shows command, packet, and agent handoff
- comparison is factual, not hostile

### M13: Install And First-Run Trust

Goal: reduce install friction.

- verify release binary workflow end-to-end
- test install script on macOS/Linux paths
- add crates.io publish checklist final pass
- add Homebrew tap instructions
- decide whether `npx scoutpack` wrapper is worth maintenance
- add `scoutpack doctor`

Acceptance:

- user can install without Rust if release exists
- `doctor` tells user what index exists, what repo type was detected, and what to do next
- docs show fastest path per OS

### M14: Agent Presets And Adoption Guides

Goal: make ScoutPack obvious for specific agent users.

- add `--agent claude`
- add `--agent codex`
- add `--agent cursor`
- add `--agent aider`
- add `--agent copilot`
- document output differences per preset
- add copy-paste MCP config snippets
- add examples for:
  - Claude Code PR review
  - Codex bugfix
  - Cursor context handoff
  - Aider file selection
  - Copilot MCP tool use

Acceptance:

- user can run one command matching their agent
- docs avoid fake integrations
- presets only tune output format, budget, verbosity, snippets, commands, and risk emphasis

### M15: OSS Launch Package

Goal: reach actual audience without generic posting.

- submit to MCP directories and awesome lists
- add launch post draft
- add issue labels and good-first-issue list
- add GitHub Discussions note in contributing docs
- add benchmark/demo assets to README
- publish reproducible example packets
- create short "review my PR with ScoutPack" guide

Acceptance:

- repo has clear contribution path
- launch material links to reproducible commands
- outreach targets high-intent places: MCP, Claude Code, Aider, Cursor, Copilot communities

### M16: Monorepo And Workspace Support

Goal: support larger repos after launch surface is clear.

- detect package boundaries:
  - `pnpm-workspace.yaml`
  - `turbo.json`
  - `nx.json`
  - package.json workspaces
  - Cargo workspaces
  - `go.work`
- show likely workspace/package in context packets
- suggest package-scoped test/lint/build commands
- add `--workspace <name>` filter if needed

Acceptance:

- context packet says likely affected package/app
- monorepo fixture has integration tests
- no large rewrite of scanner/index contracts

## Deferred

These stay deferred until adoption signal:

- deeper call graph across imports
- full symbol graph UI
- remote repo processing
- editor extension
- hosted playground
- default semantic search
- cloud sync

## Kill Criteria

Pause or simplify if a focused launch produces:

- no external users
- no external issues or discussions
- no MCP or agent-tooling interest
- usage only asks for whole-repo packing
- maintenance cost exceeds personal value

Continue if:

- users compare ScoutPack to Repomix, Aider, Cursor, or Sourcegraph
- users ask for MCP configs, presets, install help, or examples
- PR review or bugfix workflow gets repeated use
- external users submit issues or PRs
