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

## Install From Release Binary

[v0.1.1](https://github.com/theamodhshetty/ScoutPack/releases/tag/v0.1.1) includes default-feature binaries and SHA-256 sidecars for five targets. Native CI extracts each archive and runs version, pack, context and doctor on Ubuntu 22.04 x86_64/arm64, macOS 15 Intel, macOS 14 Apple Silicon and Windows Server 2022 x86_64. The Unix installer was also checked on macOS arm64 against the published download. Older systems and optional semantic embeddings are not covered by these checks. See [distribution](distribution.md).

macOS/Linux users can install from GitHub Releases without a local Rust toolchain:

```bash
curl -fsSL https://raw.githubusercontent.com/theamodhshetty/ScoutPack/v0.1.1/install.sh -o /tmp/scoutpack-install.sh
sh /tmp/scoutpack-install.sh --version v0.1.1
```

The installer downloads the matching archive and sidecar, verifies SHA-256, and validates the executable before replacing an installation. These checks detect mismatches; they are not an independent publisher signature. The old `v0.1.0` release has no binary assets. From a ScoutPack checkout, inspect installer options:

```bash
sh install.sh --help
```

Install into a custom directory:

```bash
sh /tmp/scoutpack-install.sh --version v0.1.1 --dir "$HOME/.local/bin"
```

Windows users should download the x86_64 zip and adjacent `.sha256`, compare the archive hash with `Get-FileHash -Algorithm SHA256`, extract it and run `scoutpack.exe --version`. Scoop remains a template until separately verified.

## Homebrew And Scoop

Prepared templates:

- Homebrew: `dist/homebrew/scoutpack.rb`
- Scoop: `dist/scoop/scoutpack.json`

After each release, replace placeholder hashes with real SHA-256 checksums. See [distribution](distribution.md).

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
scoutpack context "describe the task here" --budget 2500
```

Queries build and refresh automatically. Use `init` for optional config/ignore files or `pack` for explicit indexing. [Getting started and troubleshooting](getting-started.md).

Generated files:

```txt
.scoutpack/
  pack.sqlite
  manifest.json
  repo-map.md
```

Optional `init` creates `scoutpack.toml` and `.scoutpackignore` separately.

`.scoutpack/` is local index data and should not be committed.

## Agent-Friendly Commands

```bash
scoutpack search "auth middleware" --json
scoutpack context "fix login redirect loop" --budget 2500 --json
scoutpack template bugfix "fix login redirect loop" --budget 2500 --json
scoutpack stats --json
```

Use JSON output when building wrappers and scripts.

For MCP clients:

```bash
scoutpack mcp .
```

See [MCP server](mcp.md) for client configuration.
