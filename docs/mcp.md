# MCP Server

ScoutPack includes a read-only MCP server for agent clients that can call tools over stdio or local Streamable HTTP/SSE.

The server never edits project files, runs project commands, or makes outbound network calls. Index-backed tools automatically build or incrementally refresh local data under `.scoutpack/` before responding:

```bash
scoutpack mcp .
```

HTTP/SSE clients can use:

```bash
scoutpack mcp . --http --port 7777
```

## Client Config

Most MCP clients use one of two local stdio config shapes.

Use an absolute project path in client configs to avoid depending on their working directory. Use the full ScoutPack executable path too if the client does not inherit your shell PATH.

Claude Code, Cursor-style, and many MCP clients:

```json
{
  "mcpServers": {
    "scoutpack": {
      "command": "scoutpack",
      "args": ["mcp", "/absolute/path/to/your-project"]
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
      "args": ["mcp", "/absolute/path/to/your-project"]
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

## HTTP/SSE Mode

Some MCP clients support Streamable HTTP instead of stdio. ScoutPack keeps stdio as the default, but can serve the same read-only tools over local HTTP/SSE:

```bash
scoutpack mcp . --http --port 7777
```

Endpoint:

```text
http://127.0.0.1:7777/mcp
```

Use `--host` only when you intentionally need a different bind address:

```bash
scoutpack mcp . --http --host 127.0.0.1 --port 7777
```

Security notes:

- Default binding is loopback-only: `127.0.0.1`.
- The server remains read-only and uses the same tools as stdio mode.
- The HTTP transport validates loopback host headers by default.
- Do not bind to a public interface unless you add your own network controls.

## Tools

| Tool | Purpose |
| --- | --- |
| `search` | ranked local index search with optional snippets |
| `context` | token-budgeted task packet, optionally with Git scope and `expand_calls` call-graph expansion |
| `template` | agent-ready prompt from a named template plus local context |
| `file_summary` | indexed metadata, symbols, and chunk ranges for one file |
| `symbol` | exact or partial symbol lookup |
| `commands` | package commands discovered during indexing, never executed |
| `recent_changes` | local git diff summary for branch, diff, or since ranges |
| `stats` | index counts and manifest metadata |

## Notes

### Commit-pinned review context

Call `context` with `{"task":"review auth changes","budget":2500,"branch":true}`.

Optional fields: `since` (Git ref string), `diff` (`base..head` string), `branch` (boolean), and `expand_calls` (hop count). Choose at most one Git scope; `branch: false` does not select a scope. Three-dot ranges are rejected: use `branch: true` for merge-base semantics.

- `branch: true` compares HEAD with its merge base against detected main/master.
- `diff: "main..HEAD"` compares explicit endpoints and uses committed HEAD content.
- `since: "main"` uses committed HEAD content and boosts changed files while retaining related context.
- Without a scope, the tool refreshes and reads current working-tree content.

Scoped packets include full resolved base/content commit IDs and exclude staged, unstaged, and untracked content. They build a temporary commit index rather than refreshing the working-tree index. Invoke the server at the Git repository root. Deleted files can appear in change summaries but have no head-content snippet. Snapshot rebuilding adds latency; commit identity is not yet a guarantee of byte-identical packets.

The response retains `task`, `budget`, `estimated_tokens`, and `packet`; commit provenance is inside Markdown, not a separate versioned evidence schema. Transport framing is outside the packet budget. Repository text remains untrusted input: clients must not treat source comments or snippets as instructions or authorize command execution from them.

### Refresh and transport

- Working-tree index-backed tools refresh changed files automatically; no separate `pack` step is required. Git-scoped `context` and `template` use temporary commit indexes.
- Refreshes only write ScoutPack-owned files under `.scoutpack/` and never modify project source.
- Tool responses include structured JSON plus text content for broad client compatibility.
- `template` supports built-in templates such as `bugfix`, `refactor`, `review`, `docs`, and `test`.
- `recent_changes` uses local git metadata only and returns file summaries with line counts, not full diff bodies.
- `recent_changes` includes `base_commit` and `head_commit` resolved commit IDs. Branch mode compares the merge base with HEAD; explicit diff mode compares the two supplied endpoints.
- Context budgets cover the complete Markdown packet under ScoutPack's estimator, excluding MCP/JSON/XML transport framing. Too-small requests return an error; bounded fallback packets disclose omitted content.
- HTTP/SSE mode uses the same tool schemas and does not add write, shell, telemetry, or network-fetch behavior.

## Client Docs

- [Claude Code MCP](https://code.claude.com/docs/en/mcp)
- [VS Code MCP configuration](https://code.visualstudio.com/docs/copilot/reference/mcp-configuration)
- [OpenAI Codex MCP overview](https://developers.openai.com/learn/docs-mcp)
