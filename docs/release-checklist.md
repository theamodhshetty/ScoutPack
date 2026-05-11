# Release Checklist

Use this checklist before tagging a public release.

## Code

- `cargo fmt --check`
- `cargo test`
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
- release tag created
- release notes include breaking changes, if any

## Crates.io Later

- verify package metadata
- verify license and README render
- `cargo publish --dry-run`
- reserve/publish crate name

