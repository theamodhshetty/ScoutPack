# ScoutPack

[![CI](https://github.com/theamodhshetty/ScoutPack/actions/workflows/ci.yml/badge.svg)](https://github.com/theamodhshetty/ScoutPack/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Offline repo context for AI agents.

ScoutPack scans a local project, builds a searchable SQLite index, and returns compact task-specific context packets for AI coding agents. No cloud. No API key. No telemetry.

## Why

AI coding agents waste tokens when they read wrong files, re-read same files, miss repo conventions, or dump broad logs into context. ScoutPack solves upstream context selection before compression.

## Exact User And Promise

ScoutPack is for developers using Codex, Claude Code, Cursor, Aider, and similar tools in medium or large repos.

Promise: get right repo context for an AI coding task in under 3 seconds after indexing, while private code stays local.

## Core Principles

- No cloud, API key, telemetry, auto-edits, or project command execution.
- Prefer deterministic local ranking before embeddings.
- Ground output in local index; say `Unknown from index` when unsure.
- Return useful markdown first; MCP comes later.

## Quick Start

```bash
cargo install --path .
scoutpack init
scoutpack pack .
scoutpack search "auth middleware" --limit 5
scoutpack context "fix login redirect loop" --budget 2500
scoutpack stats
```

ScoutPack writes local data to:

```txt
.scoutpack/
  pack.sqlite
  manifest.json
  repo-map.md
```

## Example Output

```txt
1. src/middleware/auth.ts:4-18 [function] requireAuth
   reason: matched auth in path, matched middleware in path
```

```md
# ScoutPack Context

Task:
fix login redirect loop

Relevant Files:
- `src/middleware/auth.ts`: function `requireAuth`
- `src/app/login/page.tsx`: component `LoginPage`

Current Repo Signals:
- Framework: Next.js, React
- test command: `vitest`
- lint command: `eslint .`
```

## Status

v0.1 CLI MVP in progress:

- `init`
- `pack`
- `search`
- `context`
- `stats`
- SQLite FTS5 index
- TypeScript, TSX, Markdown, JSON, YAML, TOML scanning
- package script extraction

## Roadmap

See [ROADMAP.md](ROADMAP.md).

## License

MIT. See [LICENSE](LICENSE).
