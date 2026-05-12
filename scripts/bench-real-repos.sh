#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BENCH_ROOT="${SCOUTPACK_BENCH_ROOT:-"$ROOT/.scoutpack-bench-repos"}"
RESULTS_PATH="${1:-"$ROOT/benches/RESULTS.md"}"
SCOUTPACK_BIN="${SCOUTPACK_BIN:-"$ROOT/target/release/scoutpack"}"

mkdir -p "$BENCH_ROOT" "$(dirname "$RESULTS_PATH")"

if [[ ! -x "$SCOUTPACK_BIN" ]]; then
  cargo build --release --manifest-path "$ROOT/Cargo.toml"
fi

repos=(
  "next-learn|https://github.com/vercel/next-learn.git|fix auth page routing|auth middleware"
  "full-stack-fastapi-template|https://github.com/fastapi/full-stack-fastapi-template.git|add validation to API endpoint|validation users"
  "hyperfine|https://github.com/sharkdp/hyperfine.git|improve CLI command parsing|command option"
  "chi|https://github.com/go-chi/chi.git|find middleware routing bug|middleware route"
  "changesets|https://github.com/changesets/changesets.git|update package release workflow|package release"
)

for spec in "${repos[@]}"; do
  IFS='|' read -r name url _task _query <<<"$spec"
  dest="$BENCH_ROOT/$name"
  if [[ ! -d "$dest/.git" ]]; then
    rm -rf "$dest"
    git clone --depth 1 "$url" "$dest"
  else
    git -C "$dest" fetch --depth 1 origin >/dev/null 2>&1 || true
  fi
done

python3 - "$ROOT" "$BENCH_ROOT" "$RESULTS_PATH" "$SCOUTPACK_BIN" "${repos[@]}" <<'PY'
import datetime as dt
import json
import os
import pathlib
import shlex
import statistics
import subprocess
import sys
import time

root = pathlib.Path(sys.argv[1])
bench_root = pathlib.Path(sys.argv[2])
results_path = pathlib.Path(sys.argv[3])
scoutpack = pathlib.Path(sys.argv[4])
repo_specs = [arg.split("|", 3) for arg in sys.argv[5:]]

skip_dirs = {
    ".git", ".scoutpack", "node_modules", ".next", "dist", "build", "coverage",
    ".turbo", ".cache", "target", ".venv", "vendor",
}
code_suffixes = {
    ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".py", ".rs", ".go", ".sol",
    ".md", ".json", ".yaml", ".yml", ".toml",
}

def token_estimate(text: str) -> int:
    return (len(text) + len(text.split())) // 4 + 1

def naive_tokens(repo: pathlib.Path) -> int:
    total = 0
    for path in repo.rglob("*"):
        if not path.is_file():
            continue
        if any(part in skip_dirs for part in path.parts):
            continue
        if path.suffix not in code_suffixes and path.name not in {"Pipfile", "go.mod", "requirements.txt"}:
            continue
        try:
            data = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        total += token_estimate(data)
    return total

def timed(cmd, cwd: pathlib.Path, capture=True):
    start = time.perf_counter()
    completed = subprocess.run(
        cmd,
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE if capture else subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        check=False,
    )
    elapsed_ms = (time.perf_counter() - start) * 1000
    if completed.returncode != 0:
        raise RuntimeError(
            f"command failed in {cwd}: {shlex.join(map(str, cmd))}\n"
            f"stdout:\n{completed.stdout}\nstderr:\n{completed.stderr}"
        )
    return elapsed_ms, completed.stdout

def parse_pack(output: str):
    # Indexed 41 files, reused 0 unchanged, removed 0, added 294 chunks, skipped 6 files.
    values = {"indexed": 0, "reused": 0, "chunks": 0, "skipped": 0}
    words = output.replace(",", "").replace(".", "").split()
    for idx, word in enumerate(words):
        if word == "Indexed" and idx + 1 < len(words):
            values["indexed"] = int(words[idx + 1])
        if word == "reused" and idx + 1 < len(words):
            values["reused"] = int(words[idx + 1])
        if word == "added" and idx + 1 < len(words):
            values["chunks"] = int(words[idx + 1])
        if word == "skipped" and idx + 1 < len(words):
            values["skipped"] = int(words[idx + 1])
    return values

def generated_skipped(repo: pathlib.Path) -> str:
    repo_map = repo / ".scoutpack" / "repo-map.md"
    if not repo_map.exists():
        return "unknown"
    text = repo_map.read_text(encoding="utf-8", errors="ignore")
    generated_dirs = [name for name in skip_dirs if (repo / name).exists()]
    if not generated_dirs:
        return "n/a"
    leaked = [name for name in generated_dirs if f"- `{name}/" in text]
    return "yes" if not leaked else "no"

rows = []
for name, url, task, query in repo_specs:
    repo = bench_root / name
    subprocess.run([str(scoutpack), "init"], cwd=repo, check=True, stdout=subprocess.DEVNULL)
    subprocess.run(["rm", "-rf", ".scoutpack"], cwd=repo, check=True)
    subprocess.run([str(scoutpack), "init"], cwd=repo, check=True, stdout=subprocess.DEVNULL)

    cold_ms, cold_out = timed([str(scoutpack), "pack", "."], repo)
    cold = parse_pack(cold_out)
    incremental_ms, incremental_out = timed([str(scoutpack), "pack", "."], repo)
    incremental = parse_pack(incremental_out)
    context_ms, packet = timed([str(scoutpack), "context", task, "--budget", "2500"], repo)
    packet_tokens = token_estimate(packet)
    naive = naive_tokens(repo)

    search_times = []
    for _ in range(20):
        elapsed_ms, _ = timed([str(scoutpack), "search", query, "--limit", "5"], repo)
        search_times.append(elapsed_ms)
    p50 = statistics.median(search_times)
    p95 = sorted(search_times)[int(len(search_times) * 0.95) - 1]

    rows.append({
        "name": name,
        "cold_ms": cold_ms,
        "incremental_ms": incremental_ms,
        "packet_tokens": packet_tokens,
        "naive_tokens": naive,
        "reduction": (naive / packet_tokens) if packet_tokens else 0,
        "p50": p50,
        "p95": p95,
        "files": cold["indexed"],
        "skipped": cold["skipped"],
        "reused": incremental["reused"],
        "generated_skipped": generated_skipped(repo),
        "url": url,
    })

now = dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
commit = subprocess.check_output(["git", "rev-parse", "--short", "HEAD"], cwd=root, text=True).strip()
lines = [
    "# ScoutPack Benchmark Results",
    "",
    f"Generated: {now}",
    f"ScoutPack commit: `{commit}`",
    "",
    "Methodology:",
    "- `scripts/bench-real-repos.sh` clones each public repo with `--depth 1`.",
    "- Cold index removes `.scoutpack`, runs `scoutpack init`, then `scoutpack pack .`.",
    "- Incremental re-pack runs `scoutpack pack .` again without file changes.",
    "- Packet tokens use ScoutPack's approximation: `(chars + words) / 4 + 1`.",
    "- Naive tokens count UTF-8 source/config/docs files under supported extensions while skipping generated and sensitive default folders.",
    "- Search latency runs 20 searches per repo and reports p50/p95 wall-clock time.",
    "",
    "| Repo | Cold index | Incremental re-pack | Files scanned | Files skipped | Generated ignored | Packet tokens | Naive tokens | Reduction | Search p50 | Search p95 |",
    "| --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |",
]
for row in rows:
    lines.append(
        f"| [{row['name']}]({row['url']}) | "
        f"{row['cold_ms'] / 1000:.2f}s | "
        f"{row['incremental_ms'] / 1000:.2f}s | "
        f"{row['files']} | {row['skipped']} | {row['generated_skipped']} | "
        f"{row['packet_tokens']:,} | {row['naive_tokens']:,} | "
        f"{row['reduction']:.1f}x | {row['p50']:.1f}ms | {row['p95']:.1f}ms |"
    )
lines.extend([
    "",
    "Interpretation:",
    "- These are workflow benchmarks, not academic retrieval benchmarks.",
    "- Results vary by hardware, filesystem cache, repo checkout shape, and git/network state.",
    "- ScoutPack does not run project commands during measurement.",
])
results_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(results_path)
PY
