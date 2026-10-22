# Distribution

ScoutPack supports source installs. Release candidates must pass the artifact gates below before binary installation is advertised.

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

The workflow verifies native binaries on Linux x86_64/arm64, macOS Intel/Apple Silicon, and Windows x86_64. Linux builds use Ubuntu 22.04 (glibc 2.35); macOS builds use macOS 15 Intel and macOS 14 Apple Silicon. These runner checks do not establish compatibility with older systems, optional semantic embeddings, or every workflow.

Pull requests changing the release pipeline run all five packaging checks. Manual dispatch is a preflight that uploads artifacts without creating a release. A `v*` tag must match `Cargo.toml` and creates a **draft** release only after every platform passes. Each archive has an adjacent `.sha256` file. `scripts/release.py verify` checks its hash, validates archive members, then runs the extracted binary's version, pack, context and doctor commands on a temporary fixture.

Before publishing the draft, download its artifacts, verify checksums and perform a real installer smoke check. Record the tag, source SHA, workflow run and platform results in the release notes. Preserve failed checks and unsupported-platform limitations. Homebrew/Scoop remain templates until their separate installation checks pass.

## Install Script

`install.sh` detects macOS/Linux architecture, downloads the archive and its `.sha256` sidecar from GitHub Releases, verifies SHA-256 before extracting, checks the executable, and replaces an existing installation only after validation. It requires `sha256sum` or `shasum`. Matching hashes detect corrupt/mismatched downloads; they are not independent publisher signatures. It installs `scoutpack` to:

1. `SCOUTPACK_INSTALL_DIR`, if set
2. `/usr/local/bin`, when writable
3. `$HOME/.local/bin`, otherwise

Examples:

```bash
curl -fsSL https://raw.githubusercontent.com/theamodhshetty/ScoutPack/main/install.sh | sh
curl -fsSL https://raw.githubusercontent.com/theamodhshetty/ScoutPack/main/install.sh | sh -s -- --version v0.1.1
curl -fsSL https://raw.githubusercontent.com/theamodhshetty/ScoutPack/main/install.sh | sh -s -- --dir "$HOME/.local/bin"
```

The CI workflow runs offline installer regressions (valid upgrade, corruption, missing/malformed checksums, paths with spaces and the macOS hashing fallback), plus `bash -n install.sh` and `sh install.sh --help`.

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
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo package --locked
python3 -m unittest discover -s scripts/tests -p 'test_*.py'
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
