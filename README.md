<p align="center">
  <img src="assets/scoutpack-logo.svg" width="720" alt="ScoutPack logo">
</p>

<p align="center">
  <strong>Offline repo context compiler for AI coding agents.</strong>
</p>

<p align="center">
  Build a local searchable index, then hand Codex, Claude Code, Cursor, Aider, and other agents only the files, symbols, snippets, commands, and risks that matter.
</p>

<p align="center">
  <a href="https://github.com/theamodhshetty/ScoutPack/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/theamodhshetty/ScoutPack/ci.yml?branch=main&label=CI"></a>
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-2563eb"></a>
  <a href="Cargo.toml"><img alt="Rust" src="https://img.shields.io/badge/Rust-CLI-f97316"></a>
  <a href="docs/mcp.md"><img alt="MCP" src="https://img.shields.io/badge/MCP-read--only-14b8a6"></a>
  <img alt="Local first" src="https://img.shields.io/badge/local--first-no%20cloud-0f172a">
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> ·
  <a href="#demo">Demo</a> ·
  <a href="#mcp-server">MCP</a> ·
  <a href="#commands">Commands</a> ·
  <a href="docs/installation.md">Install</a> ·
  <a href="docs/milestones.md">Roadmap</a>
</p>

---

## Why ScoutPack

AI coding agents are powerful, but they still burn time and tokens when they start with the wrong repo context.

| Without ScoutPack | With ScoutPack |
| --- | --- |
| Agent scans broad folders. | Agent starts with ranked files and symbols. |
| Repo conventions get missed. | Package scripts and framework signals are included. |
| Secret files are risky. | Sensitive patterns are skipped by default. |
| Context grows into noise. | Packets stay under a token budget. |
| Debugging starts from guesses. | Risks include source-backed evidence. |

ScoutPack answers one practical question before edits start:

```txt
What should this AI agent read first for this task?
```

## One-Minute Flow

```bash
cargo install --git https://github.com/theamodhshetty/ScoutPack.git

cd your-project
scoutpack init
scoutpack pack .
scoutpack context "fix login redirect loop" --budget 2500
```

Output is local, compact, and agent-ready:

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
- build command: `next build`

Risks:
- redirect loop if post-login destination points back to login or auth guard
  (source: `src/middleware/auth.ts:3-14`)
```

No cloud. No API key. No telemetry. No auto-edits. No project command execution.

## What You Get

| Feature | What it does |
| --- | --- |
| Local index | SQLite + FTS5 index in `.scoutpack/` |
| Smart scan | Respects `.gitignore`, `.scoutpackignore`, binary limits, and sensitive skips |
| Code structure | TypeScript/TSX symbols, route handlers, imports, Markdown sections, config chunks |
| Task context | Token-budgeted packet with relevant files, snippets, commands, and risks |
| MCP server | Read-only `search`, `context`, `file_summary`, `symbol`, `commands`, `stats` tools |
| Automation output | JSON mode for wrappers and scripts |

## Search Terms

People may look for this as AI coding agent context, local codebase search for AI, offline repo indexing, context engineering for code, local-first RAG alternative, token-budgeted code context, repo map, MCP code search, or private repo context.

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

## Demo

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

## MCP Server

Use ScoutPack directly from MCP-aware agent clients:

```json
{
  "mcpServers": {
    "scoutpack": {
      "command": "scoutpack",
      "args": ["mcp", "."]
    }
  }
}
```

Available tools:

| Tool | Returns |
| --- | --- |
| `search` | ranked files, symbols, snippets, and reasons |
| `context` | full task packet with estimated token count |
| `file_summary` | indexed file metadata, symbols, and chunk ranges |
| `symbol` | exact or partial symbol matches |
| `commands` | discovered package scripts without running them |
| `stats` | index counts and manifest details |

Full setup: [docs/mcp.md](docs/mcp.md).

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
