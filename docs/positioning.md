# Positioning

ScoutPack helps AI coding agents start with the right files, not the whole repo.

## Category

Context preflight layer for AI coding agents.

ScoutPack runs before the agent edits. It scans a local repo, builds a local index, and produces compact, task-specific context packets.

## One-Line Pitch

ScoutPack creates local, explainable, token-budgeted context packets for Claude Code, Codex, Cursor, Aider, Copilot, and MCP clients.

## Core Problem

AI coding agents often fail because they read the wrong files first.

Common failure pattern:

- agent starts from README or broad folders
- agent misses middleware, tests, config, and helper files
- agent spends tokens exploring instead of solving
- agent edits without seeing repo conventions
- user has to manually guide file selection

ScoutPack exists to reduce that setup cost.

## Product Boundary

ScoutPack is:

- local-first
- offline-friendly
- read-only
- deterministic by default
- explainable
- token-budgeted
- compatible with terminal, files, and MCP

ScoutPack is not:

- an AI coding agent
- a cloud code indexer
- an IDE replacement
- a Sourcegraph replacement
- a full static-analysis platform
- a whole-repo dump tool
- a secret scanner

## Differentiation

| Tool | Strong at | ScoutPack difference |
| --- | --- | --- |
| Repomix | packing full repos into AI-friendly files | ScoutPack builds task-specific packets under a budget |
| Aider repo-map | helping Aider navigate code while editing | ScoutPack works before any agent edits and supports multiple clients |
| Cursor indexing | IDE-native semantic context | ScoutPack is local, explicit, CLI/MCP-friendly, and inspectable |
| Sourcegraph/Cody-style code graph | enterprise code intelligence | ScoutPack is lightweight, local, OSS, and task-packet oriented |
| Filesystem MCP | letting agents read files | ScoutPack ranks files, symbols, snippets, commands, risks, and recent changes |

## Target Users

Primary:

- Claude Code users
- Codex users
- Cursor users who want explicit context control
- Aider users who want preflight file selection
- GitHub Copilot agent-mode users
- OSS maintainers reviewing PRs with AI

Secondary:

- privacy-sensitive teams
- local LLM users
- AI workflow builders
- MCP server users
- developer-tool makers who need context packets as data

## Messaging

Use:

- "context preflight"
- "right files first"
- "local repo context compiler"
- "token-budgeted context packet"
- "read-only MCP server"
- "explainable file selection"

Avoid:

- "AI agent"
- "autonomous coding"
- "magic repo understanding"
- "perfect code graph"
- "enterprise search replacement"
- "RAG platform"

## Website/README First Screen

Headline:

```txt
AI coding agents fail when they read the wrong files first.
```

Subhead:

```txt
ScoutPack is a local context preflight layer that gives Claude Code, Codex, Cursor, Aider, Copilot, and MCP clients ranked files, symbols, snippets, commands, risks, and recent changes before edits start.
```

Call to action:

```bash
scoutpack context "fix login redirect loop" --budget 2500
```

Proof points:

- no cloud calls
- no telemetry
- no auto-edits
- skips sensitive files
- local SQLite index
- markdown/json/xml/MCP output

## Adoption Wedge

Best first workflow:

```bash
scoutpack pack .
scoutpack context "review my PR" --branch --expand-calls 1 --budget 3000
```

Why this wedge works:

- PR review is common
- branch diff narrows scope
- agent needs changed files plus related helpers/tests
- output is easy to inspect
- result can be pasted into any agent

## Launch Thesis

ScoutPack should earn adoption by proving one thing:

> For real coding tasks, ScoutPack gets the agent to relevant files faster than manual exploration or whole-repo dumping.

Marketing should show before/after packets, not broad claims.

Best assets:

- terminal GIF
- real PR review demo
- Repomix/Aider/Cursor comparison
- MCP config snippets
- benchmark table with method
- examples users can reproduce locally
