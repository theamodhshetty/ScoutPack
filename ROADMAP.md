# Roadmap

Updated October 1, 2026. The active plan below supersedes the launch-first roadmap preserved later in this file. Status is planned unless completion evidence is recorded in [milestones](docs/milestones.md).

## Active Plan

ScoutPack prepares local, inspectable code context for agent-assisted review. Repeatable PR review across agent clients is the next hypothesis. Reproducible snapshots and better review outcomes remain objectives, not shipped guarantees.

| Priority | Milestone | Exit gate |
| --- | --- | --- |
| P0 | M12: Output correctness | Successful packets fit documented estimator budget; tiny budgets fail clearly; deterministic and privacy-boundary regression tests |
| P0 | M13: Review evidence | Explicit content identity, PR merge-base semantics, versioned structured output, and CLI/MCP Git scope parity |
| P1 | M14: Usable pilot | Verified release artifacts and installation; tested Codex and Claude Code workflows |
| P1 | M15: Measured usefulness | Final-packet evaluation and repeated external use; native-agent comparison with costs and uncertainty |
| P2 | M16: Distribution and decision | Reproducible case study; expansion justified by pilot evidence |

### Current Correctness Work

Whole-packet budget enforcement is now implemented with bounded fallback and an explicit too-small error. Git-scoped packets now use commit-only temporary indexes and include resolved commit IDs; branch scope uses merge base. September 25 audit reproduced a 100-token request returning 662 estimated tokens; regression coverage now guards against returning oversized output. Remaining M12/M13 work is tracked separately below and in milestones.

1. Add a regression fixture including commands, hints, and oversized metadata.
2. Define payload versus JSON/XML wrapper budget scope and name the estimator.
3. Budget every section, including task, paths, and summaries. Reject requests below minimum useful response size instead of returning oversized packets.
4. Cover long tasks, many commands, Git summaries, Unicode, empty results, and small/default budgets.
5. Update format docs and benchmark expectations with the actual contract.

Package selection, structured evidence records, broader deterministic replay, and tokenizer improvements remain separate changes. Git-scoped semantic search is currently unsupported; temporary snapshot indexing adds latency. Runtime stays local-first, read-only toward project files, and deterministic by default; optional semantic inference stays opt-in.

### Evidence And Efficiency

Scope commands/frameworks to selected packages. Label unresolved symbol relationships. Measure unchanged refresh, incremental refresh, retrieval, and full default command latency separately. Optimize directory pruning and SQL queries where profiling supports it. Evaluate expected files and useful ranges in final packets, not search results alone.

Release scripts and formula templates are scaffolding until artifacts and installation pass verification. Last inspected public release was v0.1.0 without binaries; recheck remote state before release claims. Version numbers follow actual compatibility changes and publication history.

### Validation And Maintenance

Start a six-week validation window when execution begins; cap effort around 8-12 maintainer hours weekly. Earlier calendar dates were estimates, not completed recruitment or delivery. Keep one feature PR in flight, with tests, formatting, strict Clippy, docs, changelog, and CI review before release.

Proposed demand targets: five external users complete a real review; three return for at least three tasks over two weeks; support fits the effort cap. These are targets, not results. Set a monetary cap before any separately invoked, opt-in external-model evaluation. Continue for measured time/cost benefits without unacceptable correctness loss, or a demonstrated repeatability need. Installation failures require repair before judging demand.

Weekly: triage privacy, incorrect evidence, installation, then measured relevance/performance failures; reserve one pilot conversation. Monthly: dependencies, client compatibility, and release/doc consistency. If usable workflows attract no repeat use, narrow to stable CLI/library maintenance.

Deferred: preset matrix, more embedding infrastructure/languages, full LSP, huge-monorepo claims, hosted MCP, cross-agent memory, new branding, and broad directory submissions. New business ideas belong in separate project decisions.

Rationale and source-level findings: [September strategy review](docs/strategy-review-2026-09-25.md).

## Superseded Roadmap

The following is historical planning. Its v0.3-v0.5 sequence and kill criteria are replaced by the active plan above; feature descriptions do not establish verified guarantees.

ScoutPack is a context preflight layer for AI coding agents.

Core promise:

> Help agents start with the right files, not the whole repo.

ScoutPack should stay local-first, read-only, deterministic by default, and useful before an agent edits code.

## Positioning

ScoutPack is not an AI coding agent, IDE, or whole-repo packer.

It is an offline repo context compiler that answers:

```txt
What should this AI agent read first for this task?
```

Primary audience:

- developers using Claude Code, Codex, Cursor, Aider, Copilot, or MCP clients
- OSS maintainers reviewing PRs with AI help
- teams that cannot send broad repo context to cloud indexers
- agent power users who want explicit, inspectable context

## Completed Foundation

Current repo already has the foundation needed for credible early adoption:

- Rust CLI with `init`, `pack`, `search`, `context`, `template`, `watch`, `stats`, and MCP
- local SQLite + FTS index
- secret/generated-file skip rules
- token-budgeted markdown, JSON, and XML context output
- JavaScript, TypeScript, Python, Rust, Go, and Solidity symbol extraction
- git-aware context with `--since`, `--diff`, and `--branch`
- read-only MCP over stdio and HTTP/SSE
- optional local semantic search behind a Cargo feature
- prompt templates for common agent tasks
- benchmark framework and reproducible results
- basic direct-call expansion with `context --expand-calls`
- release/install scaffolding

## Current Strategy

Stop prioritizing broad feature growth. Shift to adoption proof.

Next work should make ScoutPack easier to understand, install, demo, and compare.

Success for the next phase is not "more features"; it is:

- user understands the project in 30 seconds
- install path works without reading source
- demo shows a real agent getting better context
- MCP users can configure it quickly
- maintainers can compare it honestly against Repomix, Aider repo-map, Cursor, and Sourcegraph

## v0.3: Launch Readiness

Goal: make ScoutPack easy to try and easy to trust.

### P0: Product Surface

- tighten README around "context preflight"
- add 30-second terminal GIF or asciinema
- add real task demo using a small public fixture repo
- add comparison page:
  - ScoutPack vs Repomix
  - ScoutPack vs Aider repo-map
  - ScoutPack vs Cursor indexing
  - ScoutPack vs Sourcegraph/Cody-style code graph
- add `docs/positioning.md`
- add `docs/demo.md`
- add `docs/comparison.md`

### P1: Install And First Run

- verify GitHub release binaries end-to-end
- make install script visible and documented
- prepare crates.io publish checklist
- add Homebrew tap instructions
- investigate optional `npx scoutpack` wrapper
- validate the shipped `scoutpack doctor` across release binaries

### P2: Agent Adoption

- add agent presets:
  - `--agent claude`
  - `--agent codex`
  - `--agent cursor`
  - `--agent aider`
  - `--agent copilot`
- submit/readiness docs for MCP directories and awesome lists
- add copy-paste config snippets per client
- add one example per workflow:
  - PR review
  - bugfix
  - refactor
  - security audit
  - onboarding

## v0.4: Workflow Depth

Goal: make ScoutPack more useful on real work branches.

- metadata-aware automatic index refresh is shipped; benchmark it on larger monorepos
- `scoutpack explain <file>`
- `scoutpack eval`
- better monorepo/workspace summaries
- workspace-scoped commands and package boundaries
- more stable JSON schema docs
- package-level likely test/build/lint commands
- call graph improvements across imports
- risk profiles for Next.js, FastAPI, Solidity, and auth/session flows

## v0.5: Distribution And Ecosystem

Goal: remove install friction and meet users where agents live.

- crates.io release
- Homebrew tap
- Scoop manifest
- optional npm wrapper
- MCP registry submissions
- GitHub Action for context packet generation
- template gallery
- public examples from real OSS issues

## Later

Only pursue after adoption signal:

- stronger semantic ranking
- deeper language parsers
- persistent multi-repo workspace graph
- editor extensions
- remote repo processing
- hosted docs/search playground

## Kill Criteria

Pause or radically simplify if, after a focused launch:

- no real external users appear
- no external issues or discussions appear
- MCP/tooling communities do not care
- users only ask for whole-repo packing, where Repomix already wins
- maintenance cost exceeds personal usage value

Continue if:

- users compare ScoutPack to Repomix, Aider, Cursor, or Sourcegraph unprompted
- developers use it in PR review or bugfix workflows
- agent users ask for presets, MCP configs, or install help
- one workflow becomes repeatedly useful

See [docs/milestones.md](docs/milestones.md) for execution tracking.
