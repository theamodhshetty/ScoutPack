# Getting Started With ScoutPack

Goal: inspect a useful local code-context packet and hand it to your existing coding agent. No API key or model setup is required in default mode.

## Install And Check

With Rust/Cargo and Git installed:

```bash
cargo install --git https://github.com/theamodhshetty/ScoutPack.git --locked
scoutpack --version
scoutpack --help
```

Using a checkout? Run `cargo install --path . --locked` from the ScoutPack checkout, then switch to the project you want to inspect. [Installation details](installation.md).

## Choose The Content You Need

### Current Work, Including Uncommitted Edits

Run from your project's root:

```bash
scoutpack context "fix login redirect loop" --budget 2500
```

Use a concrete symptom, component, or behavior. Ordinary queries create and refresh `.scoutpack/` automatically. `scoutpack init` is optional configuration.

Inspect selected paths and snippets, then paste the packet into your agent with the task. Confirm evidence is relevant; native search can fill gaps.

### Committed PR Review

```bash
scoutpack context "review my PR" --branch --budget 3000
```

This reads committed HEAD content and the merge base with detected main/master. Dirty edits are excluded. Full commit IDs identify the content; ranges may differ from open working-tree files. Explicit endpoints use `--diff base..head`. [Git scope and limitations](../examples/git-aware-context.md).

## Configure What Gets Indexed

```bash
scoutpack init
```

Creates configuration and ignore files without replacing existing ones. Add private project-specific paths to `.scoutpackignore`, for example:

```gitignore
private-notes/
customer-exports/
internal-plan.md
```

Common sensitive/generated patterns are already excluded. Add `.scoutpack/` to your project's `.gitignore` if needed. Config and ignore files can be shared when appropriate; generated indexes should stay local.

Review output before sharing. Pattern filters cannot guarantee every secret is detected. [Security policy](../SECURITY.md).

## Optional Next Steps

| Need | Command or guide |
| --- | --- |
| Find code | `scoutpack search "auth middleware" --limit 5` |
| Output for a script | `scoutpack context "fix login" --budget 2500 --format json` |
| Add task framing | `scoutpack template bugfix "signup returns 500" --budget 2500` |
| Keep index fresh during editing | `scoutpack watch .` |
| Connect an agent directly | [MCP setup](mcp.md) |
| Inspect index counts | `scoutpack stats` |

## Troubleshooting

| Symptom | Next action |
| --- | --- |
| `cargo: command not found` | Install Rust through [rustup](https://rustup.rs/). Current source install requires Cargo. |
| `scoutpack: command not found` | Check Cargo binary directory is on PATH, usually `$HOME/.cargo/bin`; reopen terminal after toolchain setup. |
| Budget too small | Use `--budget 2500` or shorten task. Required task/revision metadata cannot be silently discarded. |
| Missing main branch | Use `--diff <base>..<head>` with refs available locally. ScoutPack does not fetch remote refs. |
| Git command run below repository root | Run from root; `git rev-parse --show-toplevel` identifies it. |
| Expected file absent | Check ignore rules, language support, size limit, and whether requested commit contains it. |
| Few useful results | Use concrete path, symbol, and symptom terms; inspect `search` results. |
| Unrelated suggested commands | Detection can span packages. Verify selected package before executing a command. |
| Corrupt or stale index | Run `scoutpack doctor`, then `scoutpack doctor --fix` when repair is recommended. |
| MCP searches wrong project | Use absolute repository path in server args; client's working directory may differ. |
| Semantic search with Git scope | Omit `--semantic`; commit snapshots currently use lexical retrieval. |

`--no-refresh` reuses an ordinary index and can be stale. It is not a reproducibility lock. Git-scoped packets always build their own temporary index.

For help include OS, `scoutpack --version`, installation source, command, and a public minimal fixture. Remove credentials and private context. [Bug report](https://github.com/theamodhshetty/ScoutPack/issues/new?template=bug_report.yml).
