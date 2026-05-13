# Distribution

ScoutPack supports source installs today and is prepared for binary releases.

## Install Targets

| Channel | Status | Command |
| --- | --- | --- |
| GitHub source install | supported | `cargo install --git https://github.com/theamodhshetty/ScoutPack.git` |
| Local checkout install | supported | `cargo install --path .` |
| GitHub release binaries | prepared | `curl -fsSL https://raw.githubusercontent.com/theamodhshetty/ScoutPack/main/install.sh \| sh` |
| Homebrew tap | prepared | `brew install theamodhshetty/scoutpack/scoutpack` |
| Scoop bucket | prepared | `scoop install scoutpack` |
| crates.io | planned | `cargo install scoutpack` |

## Release Assets

`.github/workflows/release.yml` builds archives for:

| Target | Asset |
| --- | --- |
| macOS arm64 | `scoutpack-aarch64-apple-darwin.tar.gz` |
| macOS x86_64 | `scoutpack-x86_64-apple-darwin.tar.gz` |
| Linux x86_64 | `scoutpack-x86_64-unknown-linux-gnu.tar.gz` |
| Linux arm64 | `scoutpack-aarch64-unknown-linux-gnu.tar.gz` |
| Windows x86_64 | `scoutpack-x86_64-pc-windows-msvc.zip` |

The workflow runs on `v*` tags and can also be started manually.

## Install Script

`install.sh` detects macOS/Linux architecture, downloads the matching archive from GitHub Releases, and installs `scoutpack` to:

1. `SCOUTPACK_INSTALL_DIR`, if set
2. `/usr/local/bin`, when writable
3. `$HOME/.local/bin`, otherwise

Examples:

```bash
curl -fsSL https://raw.githubusercontent.com/theamodhshetty/ScoutPack/main/install.sh | sh
curl -fsSL https://raw.githubusercontent.com/theamodhshetty/ScoutPack/main/install.sh | sh -s -- --version v0.1.0
curl -fsSL https://raw.githubusercontent.com/theamodhshetty/ScoutPack/main/install.sh | sh -s -- --dir "$HOME/.local/bin"
```

The CI workflow checks `bash -n install.sh` and `sh install.sh --help`.

## Homebrew

Template: `dist/homebrew/scoutpack.rb`

Release steps:

1. Create GitHub release assets.
2. Compute SHA-256 checksums:

```bash
shasum -a 256 scoutpack-*.tar.gz
```

3. Replace placeholder hashes in `dist/homebrew/scoutpack.rb`.
4. Copy formula into `theamodhshetty/homebrew-scoutpack/Formula/scoutpack.rb`.
5. Test locally:

```bash
brew install --build-from-source ./Formula/scoutpack.rb
brew test scoutpack
```

## Scoop

Template: `dist/scoop/scoutpack.json`

Release steps:

1. Create GitHub Windows release asset.
2. Compute SHA-256 checksum:

```powershell
Get-FileHash .\scoutpack-x86_64-pc-windows-msvc.zip -Algorithm SHA256
```

3. Replace placeholder hash in `dist/scoop/scoutpack.json`.
4. Copy manifest into a Scoop bucket.
5. Test locally:

```powershell
scoop install .\scoutpack.json
scoutpack --version
```

## Preflight

Run before tagging:

```bash
cargo fmt --check
cargo test
cargo clippy -- -D warnings
cargo package
bash -n install.sh
sh install.sh --help
./target/release/scoutpack --help
```

## Release Notes Checklist

- note new CLI commands
- note MCP tool changes
- note privacy/security behavior
- include install and update commands
- confirm `.scoutpack/` remains local-only generated data
- include release asset checksums for package-manager updates
