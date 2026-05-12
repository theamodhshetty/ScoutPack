# Installation

ScoutPack is a Rust CLI. It runs locally and stores its index inside the repo being scanned.

## Requirements

- Rust stable
- Git, if installing from GitHub

ScoutPack bundles SQLite through `rusqlite`, so users do not need a separate SQLite install for normal use.

## Install From GitHub

```bash
cargo install --git https://github.com/theamodhshetty/ScoutPack.git
```

Default installs do not include semantic embedding dependencies.

Optional local semantic search:

```bash
cargo install --git https://github.com/theamodhshetty/ScoutPack.git --features semantic
scoutpack pack . --embed
scoutpack search "login flow" --semantic
```

First `--embed` use may download `BAAI/bge-small-en-v1.5` to `~/.cache/scoutpack/models` after confirmation. Embeddings remain local in `.scoutpack/pack.sqlite`.

Semantic builds use fastembed with dynamic ONNX Runtime loading so unsupported platform binaries do not break default installs. If your platform does not provide ONNX Runtime automatically, install ONNX Runtime and set the loader path before using `--embed`.

Check install:

```bash
scoutpack --help
```

## Install From Local Checkout

```bash
git clone https://github.com/theamodhshetty/ScoutPack.git
cd ScoutPack
cargo install --path .
```

## Update

From GitHub:

```bash
cargo install --git https://github.com/theamodhshetty/ScoutPack.git --force
```

From local checkout:

```bash
git pull
cargo install --path . --force
```

## Uninstall

```bash
cargo uninstall scoutpack
```

## Shell Completions

Generate completion scripts from the installed binary:

```bash
scoutpack completions bash
scoutpack completions zsh
scoutpack completions fish
scoutpack completions powershell
scoutpack completions elvish
```

Example for zsh:

```bash
mkdir -p ~/.zfunc
scoutpack completions zsh > ~/.zfunc/_scoutpack
```

Ensure `~/.zfunc` is in your `fpath` before `compinit`.

## First Run

Inside a project repo:

```bash
scoutpack init
scoutpack pack .
scoutpack context "describe the task here" --budget 2500
```

Generated files:

```txt
.scoutpack/
  pack.sqlite
  manifest.json
  repo-map.md
scoutpack.toml
.scoutpackignore
```

`.scoutpack/` is local index data and should not be committed.

## Agent-Friendly Commands

```bash
scoutpack search "auth middleware" --json
scoutpack context "fix login redirect loop" --budget 2500 --json
scoutpack stats --json
```

Use JSON output when building wrappers and scripts.

For MCP clients:

```bash
scoutpack pack .
scoutpack mcp .
```

See [MCP server](mcp.md) for client configuration.
