# Prompt Templates

ScoutPack templates combine a task, a context packet, and optional recent-change summary into an agent-ready prompt.

## Built-In Templates

| Name | Use |
| --- | --- |
| `bugfix` | root cause, minimal fix, regression test |
| `refactor` | behavior-preserving cleanup |
| `review` | correctness, security, performance, test coverage review |
| `docs` | docstrings, comments, or docs updates |
| `test` | focused tests for selected code paths |

## CLI

```bash
scoutpack pack .
scoutpack template bugfix "users see 500 on signup" --budget 3000
```

For review prompts with local git context:

```bash
scoutpack template review "review my branch" --branch --budget 3000
scoutpack template review "review auth changes" --diff main..HEAD
scoutpack template bugfix "continue auth fix" --since HEAD~5
```

Machine-readable output:

```bash
scoutpack template bugfix "users see 500 on signup" --budget 3000 --json
```

## Custom Templates

Custom templates are Markdown files ending in `.md`.

ScoutPack looks in this order:

1. `./scoutpack/templates/`
2. `./.scoutpack/templates/`
3. `~/.config/scoutpack/templates/`
4. built-in templates

Example:

```md
# My Prompt

Task:
{{task}}

{{recent_changes}}

{{context}}
```

Save as:

```txt
scoutpack/templates/my-prompt.md
```

Run:

```bash
scoutpack template my-prompt "fix login redirect loop" --budget 2500
```

## Placeholders

| Placeholder | Replaced with |
| --- | --- |
| `{{task}}` | raw task text |
| `{{context}}` | ScoutPack context packet |
| `{{recent_changes}}` | git summary when `--since`, `--diff`, or `--branch` is used; otherwise a short no-range note |

Unknown placeholders are left unchanged.

## MCP

The MCP server exposes a `template` tool with:

- `name`
- `task`
- `budget`
- `since`
- `diff`
- `branch`

ScoutPack still only reads local index and git metadata. It does not edit files or execute project commands.
