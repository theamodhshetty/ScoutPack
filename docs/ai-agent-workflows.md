# AI Agent Workflows

ScoutPack works with any AI coding tool that accepts text context or supports MCP tools.

## Recommended Flow

```bash
scoutpack pack .
scoutpack context "describe the task" --budget 2500
```

Paste the packet before asking the agent to edit. For repeated work, configure the MCP server once and let the client call ScoutPack tools directly.

For a complete task prompt, use a template:

```bash
scoutpack template bugfix "describe the bug" --budget 2500
```

## Client Matrix

| Client | Use ScoutPack with text | Use ScoutPack with MCP |
| --- | --- | --- |
| Codex | paste `scoutpack context ...` output into the task | configure stdio MCP if your Codex environment exposes MCP servers |
| Claude Code | paste context packet before edit request | use `.mcp.json` or `claude mcp add` |
| GitHub Copilot in VS Code | paste packet into Copilot Chat | use `.vscode/mcp.json`, Agent mode, tools picker |
| Cursor | paste packet or JSON output | use Cursor MCP config |
| Aider | use `search` output to decide files | not required |
| scripts/wrappers | use `--json` commands | call MCP tools over stdio |

## Codex

```bash
scoutpack pack .
scoutpack context "implement account settings page" --budget 3000
```

Then paste the output into Codex with the task.

## Claude Code

Text mode:

```bash
scoutpack template bugfix "fix auth middleware redirect loop" --budget 2500
```

Use the packet as the first message or as supporting context before asking Claude Code to edit.

MCP mode with project config:

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

## Cursor

Use ScoutPack for focused context when Cursor chat is pulling in too many unrelated files:

```bash
scoutpack search "billing webhook" --limit 8
scoutpack context "debug failed Stripe webhook verification" --budget 3000
```

## GitHub Copilot In VS Code

Add `.vscode/mcp.json`:

```json
{
  "servers": {
    "scoutpack": {
      "type": "stdio",
      "command": "scoutpack",
      "args": ["mcp", "."]
    }
  }
}
```

Open Copilot Chat in Agent mode, enable ScoutPack tools, then ask for repo context:

```txt
Use ScoutPack to build context for fixing the login redirect loop, then edit only the likely files.
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

The MCP server exposes `search`, `context`, `template`, `file_summary`, `symbol`, `commands`, and `stats`. See [MCP server](mcp.md).

## AGENTS.md Hint

Add this to a repo `AGENTS.md` when you want agents to remember ScoutPack:

```md
Before broad repo exploration, run ScoutPack:

- `scoutpack pack .` if `.scoutpack/pack.sqlite` is missing or stale.
- `scoutpack context "<task>" --budget 2500` for a task packet.
- `scoutpack template bugfix "<task>" --budget 2500` for an agent-ready prompt.
- Prefer ScoutPack MCP tools when available: `search`, `context`, `template`, `file_summary`, `symbol`, `commands`, `stats`.
- Do not run project scripts from ScoutPack output unless explicitly asked.
```
