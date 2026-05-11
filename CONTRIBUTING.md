# Contributing

ScoutPack is small by design. Keep changes deterministic, offline, and easy to test.

## Development Setup

```bash
git clone https://github.com/theamodhshetty/ScoutPack.git
cd ScoutPack
cargo test
```

## Local Checks

```bash
cargo fmt --check
cargo test
```

## Milestone Process

- Keep each major change tied to a milestone or issue.
- Make focused commits.
- Update docs when user-facing behavior changes.
- Run local checks before pushing.
- Wait for GitHub CI after pushing.

## Rules

- No telemetry.
- No network calls in core indexing or context generation.
- No project command execution.
- Do not index sensitive files such as `.env`, private keys, or certificates.
- Keep v0.1 focused on local context selection.

## Good First Areas

- fixture coverage for ranking behavior
- better markdown/config chunking
- clearer CLI errors
- docs examples from real repos
