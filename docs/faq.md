# FAQ

## What is ScoutPack?

ScoutPack is an offline repo context compiler for AI coding agents. It indexes a local project and returns compact task-specific context packets.

## Is ScoutPack an AI agent?

No. ScoutPack does not edit code, run commands, or call models. It selects useful context for other tools.

## Does ScoutPack upload my code?

No. ScoutPack has no cloud calls, no API key, and no telemetry. It writes a local SQLite index in `.scoutpack/`.

## Is this RAG?

Not in the usual vector-search sense. v0.1 uses SQLite FTS5, tree-sitter symbols, imports, path matches, and deterministic ranking. Local embeddings may come later only if deterministic ranking is not enough.

## Does ScoutPack run package scripts?

No. It reads package scripts from `package.json`, but it does not execute project commands.

## What languages work today?

v0.1 is strongest for TypeScript, TSX, Next.js-style repos, Markdown, JSON, YAML, and TOML.

## Why not build MCP first?

The CLI came first so the context output could stand on its own. ScoutPack now also includes a read-only MCP server for clients that can call tools directly.

## What files are skipped?

ScoutPack skips binary files, oversized files, ignored directories, and common sensitive patterns such as `.env`, `.env.*`, private keys, certificates, and provisioning profiles.
