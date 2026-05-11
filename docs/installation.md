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

Use JSON output when building wrappers, scripts, or MCP clients.
