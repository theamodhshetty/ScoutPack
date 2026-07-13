#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO="${1:-}"
TASK="${2:-review recent changes}"
RESULTS_PATH="${3:-"$ROOT/benches/LOCAL.md"}"
SCOUTPACK_BIN="${SCOUTPACK_BIN:-"$ROOT/target/release/scoutpack"}"

if [[ -z "$REPO" || "$REPO" == "-h" || "$REPO" == "--help" ]]; then
  cat <<'EOF'
Usage:
  scripts/bench-one-repo.sh /path/to/repo "task prompt" [results.md]

Environment:
  SCOUTPACK_BIN=/path/to/scoutpack  Use an existing ScoutPack binary.
  SCOUTPACK_EXPECTED_PATHS="src/a.rs;src/b.rs"  Validate expected ranked files.

Measures:
  cold index, incremental index, context packet tokens, naive source-dump tokens,
  reduction ratio, and search latency. No project commands run.
EOF
  exit 0
fi

if [[ ! -d "$REPO" ]]; then
  echo "error: repo path does not exist: $REPO" >&2
  exit 1
fi

mkdir -p "$(dirname "$RESULTS_PATH")"

if [[ ! -x "$SCOUTPACK_BIN" ]]; then
  cargo build --release --manifest-path "$ROOT/Cargo.toml"
fi

python3 - "$ROOT" "$REPO" "$TASK" "$RESULTS_PATH" "$SCOUTPACK_BIN" "${SCOUTPACK_EXPECTED_PATHS:-}" <<'PY'
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
repo = pathlib.Path(sys.argv[2]).resolve()
task = sys.argv[3]
results_path = pathlib.Path(sys.argv[4])
scoutpack = pathlib.Path(sys.argv[5]).resolve()
expected_paths = [path for path in sys.argv[6].split(";") if path]

skip_dirs = {
    ".git", ".scoutpack", "node_modules", ".next", "dist", "build", "coverage",
    ".turbo", ".cache", "target", ".venv", "vendor",
}
code_suffixes = {
    ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".py", ".rs", ".go", ".sol",
    ".md", ".json", ".yaml", ".yml", ".toml",
}
special_names = {"Pipfile", "go.mod", "requirements.txt", "setup.py", "setup.cfg"}

def token_estimate(text: str) -> int:
    return (len(text) + len(text.split())) // 4 + 1

def naive_tokens(path: pathlib.Path):
    total = 0
    files = 0
    for item in path.rglob("*"):
        if not item.is_file():
            continue
        if any(part in skip_dirs for part in item.relative_to(path).parts):
            continue
        if item.suffix not in code_suffixes and item.name not in special_names:
            continue
        try:
            data = item.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        files += 1
        total += token_estimate(data)
    return total, files

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
            f"command failed: {shlex.join(map(str, cmd))}\n"
            f"cwd: {cwd}\nstdout:\n{completed.stdout}\nstderr:\n{completed.stderr}"
        )
    return elapsed_ms, completed.stdout

def parse_pack(output: str):
    values = {"indexed": 0, "reused": 0, "chunks": 0, "skipped": 0}
    words = output.replace(",", "").replace(".", "").split()
    for idx, word in enumerate(words):
        if word == "Indexed" and idx + 1 < len(words):
            values["indexed"] = int(words[idx + 1])
        if word == "reused" and idx + 1 < len(words):
            values["reused"] = int(words[idx + 1])
        if word == "added" and idx + 2 < len(words) and words[idx + 2] == "chunks":
            values["chunks"] = int(words[idx + 1])
        if word == "skipped" and idx + 1 < len(words):
            values["skipped"] = int(words[idx + 1])
    return values

timed([str(scoutpack), "init"], repo)
subprocess.run(["rm", "-rf", ".scoutpack"], cwd=repo, check=True)
timed([str(scoutpack), "init"], repo)

cold_ms, cold_out = timed([str(scoutpack), "pack", "."], repo)
cold = parse_pack(cold_out)
incremental_ms, incremental_out = timed([str(scoutpack), "pack", "."], repo)
incremental = parse_pack(incremental_out)
context_ms, packet = timed(
    [str(scoutpack), "context", task, "--budget", "2500", "--no-refresh"], repo
)
packet_tokens = token_estimate(packet)
naive_token_count, naive_file_count = naive_tokens(repo)

search_times = []
ranked_paths = []
for _ in range(10):
    elapsed_ms, search_output = timed(
        [str(scoutpack), "search", task, "--limit", "20", "--json", "--no-refresh"], repo
    )
    search_times.append(elapsed_ms)
    if not ranked_paths:
        seen = set()
        for result in json.loads(search_output)["results"]:
            path = result["path"]
            if path not in seen:
                seen.add(path)
                ranked_paths.append(path)
p50 = statistics.median(search_times)
p95 = sorted(search_times)[int(len(search_times) * 0.95) - 1]

now = dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
commit = subprocess.check_output(["git", "rev-parse", "--short", "HEAD"], cwd=root, text=True).strip()
reduction = (naive_token_count / packet_tokens) if packet_tokens else 0
host = subprocess.check_output(["uname", "-srm"], text=True).strip()

lines = [
    "# ScoutPack Local Benchmark",
    "",
    f"Generated: {now}",
    f"Repo: `{repo}`",
    f"Task: `{task}`",
    f"ScoutPack commit: `{commit}`",
    f"Environment: `{host}` / {os.cpu_count()} logical CPUs",
    "",
    "| Metric | Value |",
    "| --- | ---: |",
    f"| Cold index | {cold_ms / 1000:.2f}s |",
    f"| Incremental index | {incremental_ms / 1000:.2f}s |",
    f"| Context packet time | {context_ms:.1f}ms |",
    f"| Files indexed | {cold['indexed']} |",
    f"| Files skipped | {cold['skipped']} |",
    f"| Chunks added | {cold['chunks']} |",
    f"| Files reused on incremental pack | {incremental['reused']} |",
    f"| Packet tokens | {packet_tokens:,} |",
    f"| Naive source/config/docs files | {naive_file_count:,} |",
    f"| Naive source/config/docs tokens | {naive_token_count:,} |",
    f"| Broad-context compression | {reduction:.1f}x |",
    f"| Search p50 | {p50:.1f}ms |",
    f"| Search p95 | {p95:.1f}ms |",
    "",
    "## Notes",
    "",
    "- Naive baseline counts UTF-8 source/config/docs files and skips common generated folders.",
    "- Token counts use ScoutPack's approximation, not a model tokenizer.",
    "- ScoutPack does not execute project commands.",
    "- This measures context size and speed, not model edit correctness.",
    "- Compression without relevant-file retrieval is not an efficiency win.",
    "",
    "## Ranked Files",
    "",
]
if ranked_paths:
    lines.extend(f"{index}. `{path}`" for index, path in enumerate(ranked_paths[:5], 1))
else:
    lines.append("No ranked files returned.")

if expected_paths:
    expected = set(expected_paths)
    lines.extend(["", "## Expected-File Check", ""])
    for limit in (1, 3, 5):
        hit = any(path in expected for path in ranked_paths[:limit])
        lines.append(f"- Top-{limit}: {'hit' if hit else 'miss'}")
    hits = len(expected.intersection(ranked_paths[:5]))
    lines.append(f"- Expected coverage @5: {hits}/{len(expected)}")
    lines.append("- Expected paths: " + ", ".join(f"`{path}`" for path in expected_paths))
results_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(results_path)
PY
