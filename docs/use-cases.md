# Use Cases

ScoutPack is for developers who use AI coding agents on real repositories and need better context selection before the agent starts editing.

## AI Coding Agents

Use ScoutPack before asking Codex, Claude Code, Cursor, Aider, or another coding agent to make a change:

```bash
scoutpack pack .
scoutpack context "fix login redirect loop" --budget 2500
```

Paste the context packet into the agent prompt. The packet points at likely edit files, repo signals, commands, snippets, and risks.

## Offline Codebase Search

Use ScoutPack as a local code search helper when you need ranked, explainable matches:

```bash
scoutpack search "auth middleware" --limit 5
```

Search is local SQLite FTS plus deterministic ranking. No embeddings or external API calls are required.

## Context Engineering

ScoutPack helps with context engineering for code tasks:

- find files likely relevant to a task
- include symbols and source ranges
- preserve command and risk hints
- keep output under a token budget
- avoid sending private repo content to a cloud index

## Team Workflows

ScoutPack can be useful in teams where:

- repos are too large for one prompt
- code must stay local
- agents keep reading low-value files
- package scripts and conventions are easy to miss
- AI-generated changes need more grounded starting context

## Local RAG Alternative

ScoutPack is not a general RAG platform. It deliberately starts with deterministic ranking instead of embeddings. This makes v0.1 easier to inspect, faster to run, and safer for private code.

