# ScoutPack Benchmark Results

Generated: 2026-07-12 03:37 UTC
ScoutPack commit: `fa178a2`
Environment: `Darwin 25.5.0 x86_64` / 11 logical CPUs
Toolchain: `rustc 1.95.0 (59807616e 2026-04-14)` / `Python 3.9.2`

## Methodology

- `scripts/bench-real-repos.sh` checks out pinned public-repo commits.
- Cold index removes `.scoutpack`, runs `scoutpack init`, then `scoutpack pack .`.
- Incremental re-pack runs `scoutpack pack .` again without file changes.
- Packet and broad-baseline tokens use ScoutPack's approximation: `(chars + words) / 4 + 1`; this is not a model tokenizer.
- Broad-baseline tokens count UTF-8 source/config/docs files under supported extensions while skipping generated and sensitive default folders.
- Search latency runs 20 JSON searches per repo and reports p50/p95 wall-clock time.
- Expected files are hand-authored inspection targets for each pinned task. Top-K is measured over unique ranked paths.

## Efficiency

Compression ratio compares broad source context with a task packet. It does not prove answer quality; retrieval table below checks whether expected files appear.

| Repo | Commit | Cold | Incremental | Indexed | Skipped | Generated ignored | Packet tokens | Broad tokens | Compression | Search p50/p95 |
| --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| [next-learn](https://github.com/vercel/next-learn.git) | `914a334` | 0.17s | 0.08s | 191 | 108 | yes | 2,475 | 144,760 | 58.5x | 24.9/25.9ms |
| [full-stack-fastapi-template](https://github.com/fastapi/full-stack-fastapi-template.git) | `13652b5` | 0.18s | 0.06s | 184 | 47 | yes | 597 | 158,044 | 264.7x | 26.7/27.6ms |
| [hyperfine](https://github.com/sharkdp/hyperfine.git) | `f12f3d9` | 0.12s | 0.04s | 56 | 12 | yes | 152 | 70,099 | 461.2x | 26.2/27.7ms |
| [chi](https://github.com/go-chi/chi.git) | `a54874f` | 0.17s | 0.05s | 87 | 10 | yes | 600 | 96,733 | 161.2x | 26.7/27.9ms |
| [changesets](https://github.com/changesets/changesets.git) | `372523f` | 0.25s | 0.07s | 210 | 26 | yes | 1,175 | 317,415 | 270.1x | 29.0/29.3ms |

## Retrieval Quality

Top-K means at least one expected file appears among first K unique paths. Coverage reports expected files found in first five paths.

| Repo | Task | Top-1 | Top-3 | Top-5 | Expected coverage @5 |
| --- | --- | ---: | ---: | ---: | ---: |
| next-learn | fix NextAuth login redirect flow | yes | yes | yes | 2/3 |
| full-stack-fastapi-template | fix user registration validation | yes | yes | yes | 2/3 |
| hyperfine | improve shell command option parsing | yes | yes | yes | 2/3 |
| chi | fix route headers middleware matching bug | yes | yes | yes | 2/2 |
| changesets | fix apply release plan package update | yes | yes | yes | 2/3 |

Expected files:
- **next-learn**: `dashboard/final-example/app/ui/login-form.tsx`, `dashboard/final-example/app/login/page.tsx`, `dashboard/final-example/auth.config.ts`
- **full-stack-fastapi-template**: `backend/app/api/routes/users.py`, `backend/app/models.py`, `backend/tests/api/routes/test_users.py`
- **hyperfine**: `src/command.rs`, `src/cli.rs`, `src/options.rs`
- **chi**: `middleware/route_headers.go`, `middleware/route_headers_test.go`
- **changesets**: `packages/apply-release-plan/src/index.ts`, `packages/apply-release-plan/src/index.test.ts`, `packages/apply-release-plan/src/version-package.ts`

## Interpretation

- These are deterministic context-selection benchmarks, not model-edit benchmarks.
- Results vary by hardware, filesystem cache, repo checkout shape, and git/network state.
- Small packets with retrieval misses are failures, not efficiency wins.
- Hand-authored expected files can be incomplete; review benchmark tasks and targets when pinned repos change.
- ScoutPack does not run project commands during measurement.
