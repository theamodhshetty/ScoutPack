# Distribution

ScoutPack distribution currently targets Cargo-based installs.

## Install Targets

| Channel | Status | Command |
| --- | --- | --- |
| GitHub source install | supported | `cargo install --git https://github.com/theamodhshetty/ScoutPack.git` |
| Local checkout install | supported | `cargo install --path .` |
| crates.io | planned | `cargo install scoutpack` |
| Homebrew | planned | `brew install scoutpack` |

## Preflight

Run before publishing a release:

```bash
cargo fmt --check
cargo test
cargo package
scoutpack completions zsh > /tmp/_scoutpack
scoutpack --help
```

## Release Notes Checklist

- note new CLI commands
- note MCP tool changes
- note privacy/security behavior
- include install and update commands
- confirm `.scoutpack/` remains local-only generated data
