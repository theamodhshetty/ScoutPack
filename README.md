# ScoutPack

<p align="center">
  <img src="assets/scoutpack-logo.svg" width="640" alt="ScoutPack: local code context for AI coding agents">
</p>

**Local code search and focused context packets for AI coding agents.**

ScoutPack indexes your repository, finds relevant files and symbols, and prepares task-specific context under an estimated token budget. Use it for PR review, bug investigation, or a handoff to Codex, Claude Code, Cursor, Aider, and other tools that accept text or MCP.

[![CI](https://github.com/theamodhshetty/ScoutPack/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/theamodhshetty/ScoutPack/actions/workflows/ci.yml)
[![MIT License](https://img.shields.io/badge/license-MIT-2563eb)](LICENSE)
[![Rust CLI](https://img.shields.io/badge/Rust-CLI-f97316)](Cargo.toml)
[![Read-only MCP](https://img.shields.io/badge/MCP-read--only-14b8a6)](docs/mcp.md)

[Quick start](#quick-start) · [Choose a workflow](#choose-a-workflow) · [MCP setup](#mcp-code-search) · [Troubleshooting](docs/getting-started.md#troubleshooting) · [Documentation](docs/README.md)

## Quick Start

Requires Rust/Cargo and Git. SQLite is bundled; no model account or API key is needed for the default install.

```bash
cargo install --git https://github.com/theamodhshetty/ScoutPack.git --locked
scoutpack --version
```

In the project you want to inspect:

```bash
scoutpack context "fix login redirect loop" --budget 2500
```

Paste the packet into your agent alongside the task. Review it before sharing private code with an external provider. Ordinary queries create and refresh the local index automatically; `init` and a manual `pack` are optional.

**Trying from a checkout?** Use `cargo install --path . --locked`. The published `v0.1.0` release has an older feature set. Binary installer, Homebrew, and Scoop scaffolding exist, but require published artifacts and checksums; Cargo source installation is the documented path today. [Installation and updates](docs/installation.md).

## Choose A Workflow

| Your task | Command | What to expect |
| --- | --- | --- |
| Find relevant code | `scoutpack search "auth middleware" --limit 5` | Ranked paths, symbols, and selection reasons |
| Investigate a bug | `scoutpack context "fix login redirect loop" --budget 2500` | Working-tree context, snippets, commands, and inspection hints |
| Review committed PR changes | `scoutpack context "review my PR" --branch --budget 3000` | Commit-only context from the merge base with main to HEAD |
| Review a specific range | `scoutpack context "review auth changes" --diff main..HEAD --budget 3000` | Two-endpoint change scope with resolved content commit |
| Prepare a complete prompt | `scoutpack template bugfix "signup returns 500" --budget 2500` | Task framing plus a context packet |
| Keep local index current | `scoutpack watch .` | Debounced indexing during edits |
| Diagnose index problems | `scoutpack doctor` | Health, freshness, and suggested repair |

Run Git review commands from repository root. They exclude staged, unstaged, and untracked edits. Use ordinary `context` for work in progress. [Git review details](examples/git-aware-context.md).

## What A Context Packet Contains

Illustrative excerpt, not a performance result:

```md
# ScoutPack Context

Task:
fix login redirect loop

Relevant Files:
- `src/app/login/page.tsx`: component `LoginPage`
- `src/lib/session.ts`: function `getSession`
- `src/middleware/auth.ts`: function `requireAuth`

Current Repo Signals:
- Framework: Next.js, React, Vitest
- test command: `vitest`

Relevant Snippets:
...
```

Packets also include likely inspection areas, heuristic risk hints, and budget information. Git-scoped packets identify full base and content commit IDs. Snippet ranges refer to that commit, which may differ from files currently open in your editor.

`--budget` bounds the **complete Markdown packet using ScoutPack's estimator**, not a provider tokenizer. JSON/XML envelopes and template instructions are outside the packet budget. Limited budgets omit content; requests below the minimum metadata size fail with an actionable error.

For scripts:

```bash
scoutpack context "fix login redirect loop" --budget 2500 --format json
scoutpack search "auth middleware" --limit 5 --json
```

Context JSON currently contains metadata and a Markdown `packet` string. It is not yet a versioned structured evidence schema. XML wrapping is also available with `--format xml`.

## MCP Code Search

ScoutPack provides a local, read-only Model Context Protocol server. Start with stdio:

```bash
scoutpack mcp /absolute/path/to/your-project
```

For clients using `mcpServers`, configure an explicit project path rather than relying on the client's working directory:

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

Available tools: `search`, `context`, `template`, `file_summary`, `symbol`, `commands`, `recent_changes`, and `stats`.

| Client | Start here |
| --- | --- |
| Codex | [Text handoff and MCP setup](docs/ai-agent-workflows.md#codex) |
| Claude Code | [Project MCP config and text handoff](docs/ai-agent-workflows.md#claude-code) |
| Cursor | [Focused task context](docs/ai-agent-workflows.md#cursor) |
| GitHub Copilot / VS Code | [VS Code MCP config](docs/ai-agent-workflows.md#github-copilot-in-vs-code) |
| Aider | [Choose files from search results](docs/ai-agent-workflows.md#aider) |

Config formats differ by client. For commit-pinned review, call MCP `context` with `{"task":"review auth changes","branch":true,"budget":2500}`. Use `diff` or `since` for explicit refs instead. Local Streamable HTTP/SSE setup and scope semantics are documented in [MCP reference](docs/mcp.md).

## Supported Languages

| Language | Indexed structure |
| --- | --- |
| JavaScript / JSX | Functions, classes, components, imports, CommonJS requires, route handlers |
| TypeScript / TSX | Functions, types, interfaces, components, imports, route handlers |
| Python | Functions, classes, imports, FastAPI-style routes; common project/dependency config |
| Rust | Functions, structs, enums, traits, impl blocks, modules, use imports |
| Go | Functions, methods, structs, interfaces, packages, imports, `go.mod` |
| Solidity | Contracts, interfaces, libraries, functions, modifiers, events, imports |
| Markdown, JSON, YAML, TOML | Document sections, config chunks, package scripts and framework signals |

Structure extraction is lightweight parsing, not full compiler or language-server analysis. Import and call expansion are heuristic; same-name relationships can be ambiguous.

## Privacy And Local Storage

- Default mode uses local SQLite/FTS5 and tree-sitter. No model calls, code uploads, or telemetry.
- ScoutPack does not edit source files or execute suggested project commands.
- It respects `.gitignore` and `.scoutpackignore`, skips common sensitive patterns, and excludes generated folders, binary files, and oversized files.
- Ordinary queries store local index data in `.scoutpack/`. Git-scoped queries build and remove a temporary commit index.
- Optional semantic builds can download a model after confirmation. They are not included in default installs.

Keep `.scoutpack/` out of version control. Sensitive-pattern filtering is not a complete secret scanner: inspect output before sharing. [Security policy](SECURITY.md) · [Ignore-file setup](docs/getting-started.md#configure-what-gets-indexed).

## When ScoutPack Helps

Use ScoutPack when you want explicit file selection, local indexing, or commit-identified context you can hand between tools. Native agent search may already be sufficient for a small repository or a simple task.

Existing [benchmark results](benches/RESULTS.md) measure retrieval and compression, not proven reductions in agent cost or time to a correct edit. Final-packet and native-agent evaluation remain planned. [Methodology](docs/benchmarks.md) · [Comparison and limitations](docs/comparison.md) · [Why this exists](WHY.md).

## Documentation And Help

| Need | Guide |
| --- | --- |
| First successful packet or troubleshooting | [Getting started](docs/getting-started.md) |
| Install, upgrade, or remove ScoutPack | [Installation](docs/installation.md) |
| Understand budgets, privacy, or Git scope | [FAQ](docs/faq.md) |
| Use built-in or custom prompts | [Templates](docs/templates.md) |
| Browse all guides | [Documentation index](docs/README.md) |
| Report a reproducible problem | [Bug report](https://github.com/theamodhshetty/ScoutPack/issues/new?template=bug_report.yml) |
| Suggest a workflow improvement | [Feature request](https://github.com/theamodhshetty/ScoutPack/issues/new?template=feature_request.yml) |

## Contributing

```bash
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

[Contributing](CONTRIBUTING.md) · [Roadmap](ROADMAP.md) · [Milestones](docs/milestones.md) · [Changelog](CHANGELOG.md)

MIT licensed. [License](LICENSE).
