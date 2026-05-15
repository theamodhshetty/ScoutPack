# Roadmap

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
- add `scoutpack doctor` for index health, repo signals, and install sanity

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

- `scoutpack doctor`
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
