# FAQ

## What is ScoutPack?

ScoutPack is an offline repo context compiler for AI coding agents. It indexes a local project and returns compact task-specific context packets.

## Is ScoutPack an AI agent?

No. ScoutPack does not edit code, run commands, or call models. It selects useful context for other tools.

## Does ScoutPack upload my code?

No. ScoutPack has no cloud calls, no API key, and no telemetry. It writes a local SQLite index in `.scoutpack/`.

## Is this RAG?

By default, no. ScoutPack uses SQLite FTS5, tree-sitter symbols, imports, path matches, and deterministic ranking.

Optional local semantic search is available only when ScoutPack is built with `--features semantic`. It stores per-chunk embeddings in the local SQLite index. First use may download `BAAI/bge-small-en-v1.5` after confirmation; there are still no cloud model APIs or telemetry.

## Does ScoutPack run package scripts?

No. It reads package scripts from `package.json`, but it does not execute project commands.

## What languages work today?

Current development code supports JavaScript/JSX, TypeScript/TSX, Python, Rust, Go, and Solidity symbols, plus Markdown, JSON, YAML, and TOML. Published v0.1.0 contains an older feature set; check installation source when comparing behavior.

## Do I need init and pack before every task?

No. Ordinary queries build and refresh automatically. `init` creates optional config/ignore files; `pack` provides explicit indexing. [Getting started](getting-started.md).

## Can ScoutPack review uncommitted work?

Ordinary `context` reads the working-tree index. Git-scoped `--branch`, `--diff`, and `--since` read committed content only, with full base/content commit IDs. Use ordinary context for staged, unstaged, or untracked work.

## Is the token budget exact?

Only relative to ScoutPack's heuristic estimator, not a provider tokenizer. Successful Markdown packets fit their estimated budget. JSON/XML envelopes and template instructions are excluded. Too-small requests fail clearly.

## Can I install without Rust?

Binary infrastructure is prepared, but last checked v0.1.0 had no binary assets. Use Cargo source installation today. Homebrew/Scoop templates are not published packages. [Installation status](installation.md).

## Does a smaller packet mean better agent results?

No. Benchmarks measure retrieval and compression; compact packets can omit necessary code/tests. Compare review outcomes and total task cost before claiming productivity gains. [Benchmark limits](benchmarks.md).

## Does MCP context support Git scope?

Yes. MCP `context` accepts optional `since`, `diff`, or `branch` fields with the same committed-content semantics as CLI. Choose only one scope. Without a scope, context uses the current working tree. [MCP reference](mcp.md).

## Where should I start if something fails?

[Troubleshooting](getting-started.md#troubleshooting), `scoutpack doctor`, and a public minimal fixture with installation source/version. Never attach private context or credentials.

## Why not build MCP first?

The CLI came first so the context output could stand on its own. ScoutPack now also includes a read-only MCP server for clients that can call tools directly.

## What files are skipped?

ScoutPack skips binary files, oversized files, ignored directories, and common sensitive patterns such as `.env`, `.env.*`, private keys, certificates, and provisioning profiles.
