# ScoutPack Benchmark Results

Generated: 2026-05-12 15:09 UTC
ScoutPack commit: `15f539c`

Methodology:
- `scripts/bench-real-repos.sh` clones each public repo with `--depth 1`.
- Cold index removes `.scoutpack`, runs `scoutpack init`, then `scoutpack pack .`.
- Incremental re-pack runs `scoutpack pack .` again without file changes.
- Packet tokens use ScoutPack's approximation: `(chars + words) / 4 + 1`.
- Naive tokens count UTF-8 source/config/docs files under supported extensions while skipping generated and sensitive default folders.
- Search latency runs 20 searches per repo and reports p50/p95 wall-clock time.

| Repo | Cold index | Incremental re-pack | Files scanned | Files skipped | Generated ignored | Packet tokens | Naive tokens | Reduction | Search p50 | Search p95 |
| --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| [next-learn](https://github.com/vercel/next-learn.git) | 0.15s | 0.05s | 191 | 108 | yes | 1,955 | 144,760 | 74.0x | 18.8ms | 19.4ms |
| [full-stack-fastapi-template](https://github.com/fastapi/full-stack-fastapi-template.git) | 0.13s | 0.05s | 184 | 47 | yes | 452 | 158,044 | 349.7x | 19.3ms | 19.7ms |
| [hyperfine](https://github.com/sharkdp/hyperfine.git) | 0.08s | 0.03s | 56 | 12 | yes | 1,717 | 70,099 | 40.8x | 19.3ms | 19.7ms |
| [chi](https://github.com/go-chi/chi.git) | 0.10s | 0.04s | 87 | 10 | yes | 336 | 96,733 | 287.9x | 19.8ms | 20.5ms |
| [changesets](https://github.com/changesets/changesets.git) | 0.43s | 0.06s | 210 | 26 | yes | 1,207 | 317,415 | 263.0x | 21.3ms | 22.3ms |

Interpretation:
- These are workflow benchmarks, not academic retrieval benchmarks.
- Results vary by hardware, filesystem cache, repo checkout shape, and git/network state.
- ScoutPack does not run project commands during measurement.
