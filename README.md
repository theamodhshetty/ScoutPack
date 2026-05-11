# ScoutPack

[![CI](https://github.com/theamodhshetty/ScoutPack/actions/workflows/ci.yml/badge.svg)](https://github.com/theamodhshetty/ScoutPack/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Offline repo context for AI agents.

ScoutPack is an offline repo context compiler for AI coding agents. It scans a local project, builds a searchable SQLite index, and returns compact task-specific context packets for Codex, Claude Code, Cursor, Aider, and similar tools.

Use it when an AI coding assistant needs to know which files, symbols, snippets, commands, and risks matter before it starts editing.

```bash
scoutpack pack .
scoutpack context "fix login redirect loop" --budget 2500
```

No cloud. No API key. No telemetry. No auto-edits. No project command execution.

## Also Known As

People may describe this problem as:

- AI coding agent context
- local codebase search for AI
- offline repo indexing
- context engineering for code
- local-first RAG alternative for private repos
- token-budgeted code context
- repo map for AI assistants

## Why ScoutPack

AI coding agents waste tokens when they:

- read broad or wrong files
- miss repo conventions
- ignore package scripts
- re-read the same context across turns
- over-index docs when source files matter more

ScoutPack solves the upstream problem: context selection. For a task, it answers:

```txt
What files, symbols, snippets, commands, and risks should an AI agent know first?
```

## Current Status

ScoutPack is early v0.1 software, but the core CLI works:

- local repo scan with `.gitignore` and `.scoutpackignore`
- SQLite + FTS5 index in `.scoutpack/`
- incremental re-indexing for unchanged files
- TypeScript/TSX parsing with tree-sitter
- Markdown, JSON, YAML, and TOML support
- package.json script and framework detection
- token-budgeted markdown context packets
- grounded risk hints with source ranges

## Install

From a local checkout:

```bash
git clone https://github.com/theamodhshetty/ScoutPack.git
cd ScoutPack
cargo install --path .
```

From GitHub:

```bash
cargo install --git https://github.com/theamodhshetty/ScoutPack.git
```

Requirements:

- Rust stable
- SQLite support is bundled through `rusqlite`
- no Node install required for ScoutPack itself

More detail: [docs/installation.md](docs/installation.md).

## Quick Start

Inside any repo:

```bash
scoutpack init
scoutpack pack .
scoutpack search "auth middleware" --limit 5
scoutpack context "fix login redirect loop" --budget 2500
scoutpack stats
```

ScoutPack writes local-only data:

```txt
.scoutpack/
  pack.sqlite
  manifest.json
  repo-map.md
```

## Example

Search:

```txt
1. src/middleware/auth.ts:3-14 [function] requireAuth
   reason: matched auth in path, matched middleware in path
```

Context packet:

```md
# ScoutPack Context

Task:
fix login redirect loop

Relevant Files:
- `src/app/login/page.tsx`: component `LoginPage`
- `src/lib/session.ts`: function `getSession`
- `src/middleware/auth.ts`: function `requireAuth`

Current Repo Signals:
- Framework: Next.js, React, Vitest
- test command: `vitest`
- lint command: `eslint .`
- build command: `next build`

Likely Edit Areas:
- `src/app/login/page.tsx:3-11`
- `src/lib/session.ts:5-7`
- `src/middleware/auth.ts:3-14`
- auth/session boundary files
- redirect and destination parameter handling

Risks:
- redirect loop if post-login destination points back to login or auth guard (source: `src/middleware/auth.ts:3-14`)
```

Full sample: [examples/context.md](examples/context.md).

## Commands

```bash
scoutpack init
```

Creates `scoutpack.toml` and `.scoutpackignore`.

```bash
scoutpack pack .
```

Scans and indexes a repo. Re-running `pack` reuses unchanged files.

```bash
scoutpack search "auth middleware" --limit 5
```

Searches the local index and returns ranked files/symbols with reasons.

```bash
scoutpack search "auth middleware" --limit 5 --json
```

Returns machine-readable JSON for scripts and tools.

```bash
scoutpack context "fix login redirect loop" --budget 2500
```

Builds a markdown packet for an AI coding task.

```bash
scoutpack context "fix login redirect loop" --budget 2500 --json
```

Returns the context packet plus metadata as JSON.

```bash
scoutpack stats
```

Shows local index counts and manifest details.

```bash
scoutpack stats --json
```

Returns index stats and manifest data as JSON.

```bash
scoutpack mcp .
```

Starts a read-only MCP server over the existing local index. Tools include `search`, `context`, `file_summary`, `symbol`, `commands`, and `stats`.

```bash
scoutpack completions zsh > _scoutpack
```

Generates shell completions for `bash`, `zsh`, `fish`, `powershell`, or `elvish`.

## Supported Today

| Area | Support |
| --- | --- |
| TypeScript / TSX | tree-sitter symbols, imports, functions, components, route handlers |
| Markdown | heading-based sections |
| JSON | package scripts, dependencies, framework signals |
| YAML / TOML | config chunks |
| Search | SQLite FTS5 plus deterministic ranking |
| Context | markdown packets with budget-aware snippets |
| MCP | read-only stdio server for agent clients |
| Distribution | Cargo install plus generated shell completions |
| Privacy | local-only index, sensitive file skips |

## Not In Scope Yet

- embeddings or vector search
- cloud sync
- GUI
- auto-edits
- running package scripts
- live file watching

## Privacy Model

ScoutPack is offline by design:

- does not send code anywhere
- does not call external APIs
- does not collect telemetry
- does not run project commands
- skips common secret files such as `.env`, private keys, certificates, and provisioning profiles

Security details: [SECURITY.md](SECURITY.md).

## Docs

- [Installation](docs/installation.md)
- [Distribution](docs/distribution.md)
- [MCP server](docs/mcp.md)
- [AI agent workflows](docs/ai-agent-workflows.md)
- [Use cases](docs/use-cases.md)
- [FAQ](docs/faq.md)
- [Design](docs/design.md)
- [Milestones](docs/milestones.md)
- [Release checklist](docs/release-checklist.md)
- [Roadmap](ROADMAP.md)
- [Changelog](CHANGELOG.md)

## Development

```bash
cargo fmt --check
cargo test
```

Contributing guide: [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT. See [LICENSE](LICENSE).
