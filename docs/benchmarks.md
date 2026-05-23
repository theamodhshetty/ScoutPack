# Benchmarks

ScoutPack benchmarks answer one narrow question:

> How much smaller and faster is a task-specific ScoutPack packet than broad repo context?

They do not claim an AI model will always produce a better edit. Edit quality depends on model, prompt, repo, tests, and user review. ScoutPack measures context selection, packet size, indexing time, and search latency.

## What We Measure

| Metric | Meaning |
| --- | --- |
| Cold index | time for first `scoutpack pack .` after removing `.scoutpack` |
| Incremental index | time for repeated `scoutpack pack .` with no file changes |
| Packet tokens | token estimate for `scoutpack context "<task>" --budget <n>` |
| Naive tokens | token estimate for broad source/config/docs dump with generated folders skipped |
| Reduction | `naive tokens / packet tokens` |
| Search latency | repeated `scoutpack search` wall-clock time |
| Skipped files | files ignored by scanner rules, `.gitignore`, `.scoutpackignore`, binary checks, or sensitive patterns |

Current reproducible results live in [benches/RESULTS.md](../benches/RESULTS.md).

## Run Real-Repo Benchmark

This clones representative public repos into `.scoutpack-bench-repos/`, builds ScoutPack, and writes `benches/RESULTS.md`.

```bash
scripts/bench-real-repos.sh
```

Use another output path:

```bash
scripts/bench-real-repos.sh /tmp/scoutpack-results.md
```

Use another cache root:

```bash
SCOUTPACK_BENCH_ROOT=/tmp/scoutpack-bench-repos scripts/bench-real-repos.sh
```

## Benchmark Your Own Repo

Use this when evaluating whether ScoutPack helps a real workflow in your codebase:

```bash
scripts/bench-one-repo.sh /path/to/repo "review my PR" /tmp/scoutpack-local-benchmark.md
```

It measures:

- cold index time
- incremental index time
- packet size for your task
- broad naive source dump size
- reduction ratio
- search latency for your task text

It does not execute project commands and does not make network calls.

## Comparison Baselines

ScoutPack should be compared against workflow alternatives, not strawmen.

| Baseline | What to compare |
| --- | --- |
| Raw source dump | token size and setup time for broad `cat`/copy-paste context |
| Repomix-style pack | whole-repo artifact size and time |
| Aider repo-map | usefulness when already using Aider edit loop |
| Native agent search | whether explicit preflight helps before Claude Code, Codex, Cursor, or Copilot starts |
| Filesystem MCP | whether ranked context beats unguided file browsing |

ScoutPack should win only where explicit, portable, local, task-specific context matters. If native agent search is enough for a small repo, ScoutPack may not be needed.

## Honest Interpretation

Good benchmark result:

- packet stays near requested budget
- selected files match likely edit/review areas
- generated and sensitive files stay skipped
- incremental pack is fast enough to rerun often
- output gives useful agent handoff without full repo dump

Bad benchmark result:

- packet misses obvious files
- risk hints repeat task wording without evidence
- reduction comes only from dropping needed context
- generated folders leak into index
- command needs too much manual ceremony

When publishing results, include repo, commit, task prompt, ScoutPack commit, OS, CPU, and exact command.
