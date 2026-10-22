# AI Agent Workflows

ScoutPack works with any AI coding tool that accepts text context or supports MCP tools.

## Recommended Flow

```bash
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
| Codex | paste `scoutpack context ...` output into the task | configure a stdio server in Codex's TOML config; see below |
| Claude Code | paste context packet before edit request | use `.mcp.json` or `claude mcp add` |
| GitHub Copilot in VS Code | paste packet into Copilot Chat | use `.vscode/mcp.json`, Agent mode, tools picker |
| Cursor | paste packet or JSON output | use Cursor MCP config |
| Aider | use `search` output to decide files | not required |
| scripts/wrappers | use `--json` commands | call MCP tools over stdio |

## Codex

```bash
scoutpack context "implement account settings page" --budget 3000
```

Then paste the output into Codex with the task.

For MCP, use a full executable path and repository path in Codex's `config.toml`:

```toml
[mcp_servers.scoutpack]
command = "/absolute/path/to/scoutpack"
args = ["mcp", "/absolute/path/to/your-project"]
required = true
```

Use the user-level `~/.codex/config.toml`, or a trusted project's `.codex/config.toml`.
Verify the entry with `codex mcp get scoutpack`. In a Codex task, ask it to call
ScoutPack `context` with `branch: true` and a budget to review committed changes.
The MCP server must start at the Git repository root for Git scopes.

### Repeatable Codex CLI smoke check

From a ScoutPack checkout, with Python 3, Git, an installed ScoutPack binary,
and an authenticated recent Codex CLI on macOS/Linux:

```bash
python3 scripts/smoke-codex.py --run \
  --scoutpack /absolute/path/to/scoutpack \
  --evidence-dir /tmp/scoutpack-codex-evidence
```

Use a recent ScoutPack source build that includes rename metadata, rather than
an older release without `previous_path` in its change summaries.
Choose a new evidence directory for each run. This is an opt-in live model call:
it uses saved Codex authentication and quota. It sends only a generated Git
fixture, not the current repository. `--model` optionally selects an available
model; otherwise the isolated CLI default applies. `--timeout` defaults to 120 seconds.
User config (including custom provider configuration) is not loaded for this check;
saved CLI authentication is still used.

The runner creates a committed rename and a dirty replacement with different
random markers. It uses one-off config overrides, `--ignore-user-config`,
`--ephemeral`, a read-only sandbox, and only two enabled MCP tools. It does not
change your Codex config. For this isolated check it explicitly approves only
`scoutpack.context` and `scoutpack.recent_changes`; other tools are not enabled.

Non-interactive MCP calls can be blocked by approval policy even when the server
connects and Codex exits successfully. The check therefore requires successful
MCP events with the expected commit, committed marker, rename paths and budget,
plus a final answer using those results. It rejects shell fallback, failed calls,
missing evidence and working-tree index/source changes. It saves a local summary
and optional raw client logs in the requested evidence directory; inspect logs
before sharing them.

Run the transcript checks offline, without model credentials or Codex:

```bash
python3 -m unittest discover -s scripts/tests -p 'test_*.py'
```

CI runs only these offline checks. A passing live run demonstrates the tested
CLI/model combination; it does not establish compatibility with every Codex
client or show better review quality. See the official [Codex MCP configuration](https://learn.chatgpt.com/docs/extend/mcp)
and [non-interactive output/approval guidance](https://learn.chatgpt.com/docs/non-interactive-mode).

Observed October 22, 2026: the runner passed with ScoutPack source
`ae9d08daa08193e7acc371ec970b8eb7c26033a5`, Codex CLI
`0.163.0-alpha.11.1`, and `gpt-6.1-sol` on macOS arm64 over stdio.
Both tool calls succeeded, the packet reported 298 estimated tokens within its
2500 budget, and the final answer identified the committed random marker,
rename paths and content commit. An initial run without per-tool approval
exited zero with both calls blocked, motivating the transcript validation.
The alpha CLI version is an observation, not a minimum-version requirement.

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
      "args": ["mcp", "/absolute/path/to/your-project"]
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
      "args": ["mcp", "/absolute/path/to/your-project"]
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

For clients that support MCP, run ScoutPack as a read-only stdio server. Index-backed tools build and refresh ScoutPack's local index automatically:

```bash
scoutpack mcp .
```

The MCP server exposes `search`, `context`, `template`, `file_summary`, `symbol`, `commands`, `recent_changes`, and `stats`. Use absolute project paths for clients. For committed review packets, call `context` with `branch: true`, `diff: "main..HEAD"`, or `since: "main"` (only one). See [MCP server](mcp.md).

## AGENTS.md Hint

Add this to a repo `AGENTS.md` when you want agents to remember ScoutPack:

```md
Before broad repo exploration, run ScoutPack:

- Ordinary ScoutPack queries build and refresh their index automatically.
- `scoutpack context "<task>" --budget 2500` for a task packet.
- `scoutpack template bugfix "<task>" --budget 2500` for an agent-ready prompt.
- Prefer ScoutPack MCP tools when available: `search`, `context`, `template`, `file_summary`, `symbol`, `commands`, `stats`.
- Do not run project scripts from ScoutPack output unless explicitly asked.
```
