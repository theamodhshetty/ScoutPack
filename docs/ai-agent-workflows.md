# AI Agent Workflows

ScoutPack works with any AI coding tool that accepts text context.

## Codex

```bash
scoutpack pack .
scoutpack context "implement account settings page" --budget 3000
```

Then paste the output into Codex with the task.

## Claude Code

```bash
scoutpack context "fix auth middleware redirect loop" --budget 2500
```

Use the packet as the first message or as supporting context before asking Claude Code to edit.

## Cursor

Use ScoutPack for focused context when Cursor chat is pulling in too many unrelated files:

```bash
scoutpack search "billing webhook" --limit 8
scoutpack context "debug failed Stripe webhook verification" --budget 3000
```

## Aider

Use search output to decide which files to add:

```bash
scoutpack search "theme provider" --limit 5
```

Then add the suggested files to Aider.

## JSON For Tools

Machine-readable output is available:

```bash
scoutpack search "auth middleware" --json
scoutpack context "fix login redirect loop" --budget 2500 --json
scoutpack stats --json
```

This is useful for scripts, wrappers, and clients that prefer direct JSON output.

## MCP Clients

For clients that support MCP, run ScoutPack as a read-only stdio server over an existing index:

```bash
scoutpack pack .
scoutpack mcp .
```

The MCP server exposes `search`, `context`, `file_summary`, `symbol`, `commands`, and `stats`. See [MCP server](mcp.md).
