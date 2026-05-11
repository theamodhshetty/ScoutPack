# Contributing

ScoutPack is small by design. Keep changes deterministic, offline, and easy to test.

## Local Checks

```bash
cargo fmt --check
cargo test
```

## Rules

- No telemetry.
- No network calls in core indexing or context generation.
- No project command execution.
- Do not index sensitive files such as `.env`, private keys, or certificates.
- Keep v0.1 focused on local context selection.

