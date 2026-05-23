# Why ScoutPack Exists

AI coding agents already search code. ScoutPack exists for cases where "let the agent search" is not enough.

## Core Bet

Agent context should be reproducible, inspectable, local, and portable across tools.

ScoutPack answers one question before edits start:

```txt
What should this agent read first for this task?
```

It builds a local index, ranks relevant files/symbols/snippets, includes likely commands and branch changes, and emits a packet that can be pasted into or served to multiple agents.

## Why Not Just Use Claude Code, Cursor, Or Aider?

Use native agent context when:

- you already trust the agent to search the repo
- you do not need reproducibility
- you work inside one tool
- the repo is small enough that extra exploration is cheap
- automatic context selection is good enough

Use ScoutPack when:

- you want the same context packet across Claude Code, Codex, Cursor, Aider, Copilot, MCP clients, or API workflows
- you need to inspect context before sending it to a model
- you want branch-aware PR context with `--branch`, `--diff`, or `--since`
- you want a deterministic local packet for review, audit, or debugging
- you cannot use cloud codebase indexing
- you want a small packet instead of a whole-repo dump

## Why Not Just Use Repomix?

Repomix is strong at packing a repository into an AI-friendly artifact.

ScoutPack is different:

- Repomix is best for broad repo context.
- ScoutPack is best for task-specific context under a token budget.
- Repomix answers "what is in this repo?"
- ScoutPack answers "what should the agent read first for this task?"

These tools can coexist.

## What ScoutPack Is Good At

- reproducible context packets
- local-first indexing
- read-only MCP tools
- branch-aware review context
- symbol/snippet selection
- package command discovery without execution
- sensitive/generated file skips
- multi-agent handoff

## What ScoutPack Is Bad At

- replacing an IDE
- replacing agent-native search loops
- whole-repo packing
- full static analysis
- security auditing guarantees
- dataflow analysis
- executing tests
- proving an edit is correct

## Risk Hints Are Not Static Analysis

ScoutPack risk hints are rule-based prompts grounded in selected files and source ranges.

They are meant to say:

```txt
Inspect this area carefully.
```

They are not meant to say:

```txt
This bug definitely exists.
```

Risk hints can be wrong, incomplete, or obvious from the task. They should help orient an agent, not replace review.

## Best Initial Workflow

ScoutPack's strongest current workflow is PR review:

```bash
scoutpack pack .
scoutpack context "review my PR" --branch --expand-calls 1 --budget 3000
```

This gives an agent:

- changed files
- called helpers
- relevant snippets
- test/lint/build commands
- risk hints
- token-budget summary

## Success Criteria

ScoutPack is worth maintaining if users adopt it for at least one repeated workflow:

- PR review context
- multi-agent handoff
- local MCP repo brain
- audit-friendly context packets
- privacy-sensitive agent setup

If users only want whole-repo packing, ScoutPack should not compete there. Repomix already does that well.
