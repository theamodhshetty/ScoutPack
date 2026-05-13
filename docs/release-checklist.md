# Release Checklist

Use this checklist before tagging a public release.

## Code

- `cargo fmt --check`
- `cargo test`
- `cargo clippy -- -D warnings`
- `cargo build --release`
- `cargo package`
- `bash -n install.sh`
- `sh install.sh --help`
- CLI smoke test on `tests/fixtures/nextjs-basic`
- `scoutpack --help`
- `scoutpack init`
- `scoutpack pack .`
- `scoutpack search "auth middleware" --limit 5`
- `scoutpack context "fix login redirect loop" --budget 2500`

## Docs

- README quick start works from a fresh clone
- install docs mention source and GitHub install
- changelog has release notes
- roadmap matches current scope
- security promises still match implementation

## GitHub

- CI green on `main`
- open issues reflect next work
- release tag created with `v*` format
- release workflow uploaded macOS, Linux, and Windows assets
- release notes include breaking changes, if any

## Package Managers

- compute SHA-256 checksums for release archives
- update `dist/homebrew/scoutpack.rb`
- test Homebrew formula in tap
- update `dist/scoop/scoutpack.json`
- test Scoop manifest on Windows
- verify `install.sh` downloads latest release

## Crates.io Later

- verify package metadata
- verify license and README render
- `cargo publish --dry-run`
- reserve/publish crate name
