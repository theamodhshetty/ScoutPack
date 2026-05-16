# Comparison

ScoutPack is a context preflight layer. It helps AI coding agents start with the right files before edits begin.

This page compares workflow fit, not product quality. Use the tool that matches the job.

## Summary

| Tool | Best fit | ScoutPack difference |
| --- | --- | --- |
| Repomix | pack a repo or folder into an AI-friendly artifact | ScoutPack returns task-specific, ranked packets under a budget |
| Aider repo-map | give Aider enough map context while Aider edits | ScoutPack is agent-agnostic and runs before edit flow starts |
| Cursor indexing | IDE-native codebase context and chat | ScoutPack is CLI/MCP-first, explicit, local, and portable |
| Sourcegraph/Cody | enterprise-scale code search and graph context | ScoutPack is lightweight, local, OSS, and packet-oriented |
| filesystem MCP servers | let agents read files on request | ScoutPack ranks files, symbols, snippets, risks, commands, and branch changes first |

## ScoutPack vs Repomix

Use Repomix when:

- you want a full repo packed into one artifact
- you want to give an AI model broad project context
- you are not focused on a narrow task or branch

Use ScoutPack when:

- you have a specific coding task
- you need a token budget
- you want ranked files and reasons
- you want branch-aware context with `--branch`, `--diff`, or `--since`
- you want command/risk hints and call expansion

ScoutPack should not copy Repomix. Repomix is strong at whole-repo packaging. ScoutPack should stay focused on task-specific preflight packets.

## ScoutPack vs Aider Repo-Map

Use Aider repo-map when:

- you are already editing with Aider
- you want Aider to maintain its own working map
- you want repo context inside Aider's loop

Use ScoutPack when:

- you want context before choosing an agent
- you use Claude Code, Codex, Cursor, Copilot, or multiple tools
- you want a standalone packet you can inspect or paste
- you want MCP tools independent from one editing agent

ScoutPack can complement Aider: run `scoutpack search` or `scoutpack context`, then add selected files to Aider.

## ScoutPack vs Cursor Indexing

Use Cursor indexing when:

- you live inside Cursor
- you want IDE-native chat and codebase context
- you prefer automatic context selection

Use ScoutPack when:

- you want explicit context control
- you work in terminal-first flows
- you need deterministic, local-first context packets
- you want the same context for multiple agents
- you want branch-aware packets outside the IDE

ScoutPack is not trying to replace Cursor. It is useful when context needs to be portable, inspectable, or agent-neutral.

## ScoutPack vs Sourcegraph/Cody-Style Code Graph

Use Sourcegraph/Cody-style tooling when:

- repo scale is enterprise-level
- code search, ownership, and graph intelligence matter across many repos
- team wants hosted or managed code intelligence

Use ScoutPack when:

- you need a lightweight local tool
- you want no runtime cloud calls
- you want an OSS CLI/MCP server
- you want focused packets, not a full platform

ScoutPack's graph should stay pragmatic: useful direct calls and import proximity, not full LSP replacement.

## ScoutPack vs Filesystem MCP

Use filesystem MCP when:

- agent should browse files directly
- user can guide paths manually
- no ranking or preflight packet is needed

Use ScoutPack MCP when:

- agent should ask for ranked context
- file selection should include reasons
- branch changes, symbols, commands, and risks matter
- sensitive/generated files should be skipped before tool use

Filesystem MCP gives file access. ScoutPack gives file prioritization.

## Positioning Rule

ScoutPack should own this sentence:

> Before the agent edits, ScoutPack tells it what to read first.

If a feature does not strengthen that sentence, defer it.
