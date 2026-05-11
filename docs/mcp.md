# MCP Server

ScoutPack includes a read-only MCP server for agent clients that can call tools over stdio.

The server does not index, edit files, run project commands, call the network, or write to the repo. Build or refresh the local index first:

```bash
scoutpack pack .
scoutpack mcp .
```

## Client Config

Most MCP clients use one of two local stdio config shapes.

Claude Code, Cursor-style, and many MCP clients:

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

VS Code / GitHub Copilot workspace config:

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

From a local checkout:

```json
{
  "mcpServers": {
    "scoutpack": {
      "command": "cargo",
      "args": ["run", "--quiet", "--", "mcp", "."]
    }
  }
}
```

Claude Code project-scoped config can live in `.mcp.json`. VS Code stores workspace MCP config in `.vscode/mcp.json`.

## Tools

| Tool | Purpose |
| --- | --- |
| `search` | ranked local index search with optional snippets |
| `context` | token-budgeted task packet |
| `file_summary` | indexed metadata, symbols, and chunk ranges for one file |
| `symbol` | exact or partial symbol lookup |
| `commands` | package commands discovered during indexing, never executed |
| `stats` | index counts and manifest metadata |

## Notes

- Run `scoutpack pack .` again after meaningful repo changes.
- MCP tools require an existing `.scoutpack/pack.sqlite`.
- Tool responses include structured JSON plus text content for broad client compatibility.

## Client Docs

- [Claude Code MCP](https://code.claude.com/docs/en/mcp)
- [VS Code MCP configuration](https://code.visualstudio.com/docs/copilot/reference/mcp-configuration)
- [OpenAI Codex MCP overview](https://developers.openai.com/learn/docs-mcp)
