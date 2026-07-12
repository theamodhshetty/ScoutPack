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
  "next-learn|https://github.com/vercel/next-learn.git|914a33426aac20f825ca52fae80b889302b92240|fix NextAuth login redirect flow|fix NextAuth login redirect flow|dashboard/final-example/app/ui/login-form.tsx;dashboard/final-example/app/login/page.tsx;dashboard/final-example/auth.config.ts"
  "full-stack-fastapi-template|https://github.com/fastapi/full-stack-fastapi-template.git|13652b51ea0acca7dfe243ac25e2bbdc066f3c4f|fix user registration validation|fix user registration validation|backend/app/api/routes/users.py;backend/app/models.py;backend/tests/api/routes/test_users.py"
  "hyperfine|https://github.com/sharkdp/hyperfine.git|f12f3d9f86f3643b3b7deace5e160b1f0f44d2b7|improve shell command option parsing|improve shell command option parsing|src/command.rs;src/cli.rs;src/options.rs"
  "chi|https://github.com/go-chi/chi.git|a54874f0e2f12647a19e82ee70dfa8185014100c|fix route headers middleware matching bug|fix route headers middleware matching bug|middleware/route_headers.go;middleware/route_headers_test.go"
  "changesets|https://github.com/changesets/changesets.git|372523f4c2ee4ffeb8330d444d47ffb6d0af5126|fix apply release plan package update|fix apply release plan package update|packages/apply-release-plan/src/index.ts;packages/apply-release-plan/src/index.test.ts;packages/apply-release-plan/src/version-package.ts"
)

for spec in "${repos[@]}"; do
  IFS='|' read -r name url commit _task _query _expected <<<"$spec"
  dest="$BENCH_ROOT/$name"
  if [[ ! -d "$dest/.git" ]]; then
    rm -rf "$dest"
    git clone --no-checkout "$url" "$dest"
  fi
  git -C "$dest" fetch --depth 1 origin "$commit"
  git -C "$dest" checkout --detach --force "$commit"
done

python3 - "$ROOT" "$BENCH_ROOT" "$RESULTS_PATH" "$SCOUTPACK_BIN" "${repos[@]}" <<'PY'
import datetime as dt
import json
import os
import platform
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
repo_specs = [arg.split("|", 5) for arg in sys.argv[5:]]

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
for name, url, expected_commit, task, query, expected_raw in repo_specs:
    repo = bench_root / name
    repo_commit = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=repo, text=True
    ).strip()
    if repo_commit != expected_commit:
        raise RuntimeError(f"{name}: expected {expected_commit}, got {repo_commit}")
    expected_paths = expected_raw.split(";")
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
    ranked_paths = []
    for _ in range(20):
        elapsed_ms, search_output = timed(
            [str(scoutpack), "search", query, "--limit", "20", "--json"], repo
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
    expected_set = set(expected_paths)
    hit_at = {
        limit: any(path in expected_set for path in ranked_paths[:limit])
        for limit in (1, 3, 5)
    }
    expected_hits_at_5 = len(expected_set.intersection(ranked_paths[:5]))

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
        "commit": repo_commit,
        "task": task,
        "ranked_paths": ranked_paths,
        "expected_paths": expected_paths,
        "hit_at": hit_at,
        "expected_hits_at_5": expected_hits_at_5,
    })

now = dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
commit = subprocess.check_output(["git", "rev-parse", "--short", "HEAD"], cwd=root, text=True).strip()
rust_version = subprocess.check_output(["rustc", "--version"], text=True).strip()
host = subprocess.check_output(["uname", "-srm"], text=True).strip()
lines = [
    "# ScoutPack Benchmark Results",
    "",
    f"Generated: {now}",
    f"ScoutPack commit: `{commit}`",
    f"Environment: `{host}` / {os.cpu_count()} logical CPUs",
    f"Toolchain: `{rust_version}` / `Python {platform.python_version()}`",
    "",
    "## Methodology",
    "",
    "- `scripts/bench-real-repos.sh` checks out pinned public-repo commits.",
    "- Cold index removes `.scoutpack`, runs `scoutpack init`, then `scoutpack pack .`.",
    "- Incremental re-pack runs `scoutpack pack .` again without file changes.",
    "- Packet and broad-baseline tokens use ScoutPack's approximation: `(chars + words) / 4 + 1`; this is not a model tokenizer.",
    "- Broad-baseline tokens count UTF-8 source/config/docs files under supported extensions while skipping generated and sensitive default folders.",
    "- Search latency runs 20 JSON searches per repo and reports p50/p95 wall-clock time.",
    "- Expected files are hand-authored inspection targets for each pinned task. Top-K is measured over unique ranked paths.",
    "",
    "## Efficiency",
    "",
    "Compression ratio compares broad source context with a task packet. It does not prove answer quality; retrieval table below checks whether expected files appear.",
    "",
    "| Repo | Commit | Cold | Incremental | Indexed | Skipped | Generated ignored | Packet tokens | Broad tokens | Compression | Search p50/p95 |",
    "| --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |",
]
for row in rows:
    lines.append(
        f"| [{row['name']}]({row['url']}) | "
        f"`{row['commit'][:7]}` | "
        f"{row['cold_ms'] / 1000:.2f}s | "
        f"{row['incremental_ms'] / 1000:.2f}s | "
        f"{row['files']} | {row['skipped']} | {row['generated_skipped']} | "
        f"{row['packet_tokens']:,} | {row['naive_tokens']:,} | "
        f"{row['reduction']:.1f}x | {row['p50']:.1f}/{row['p95']:.1f}ms |"
    )
lines.extend([
    "",
    "## Retrieval Quality",
    "",
    "Top-K means at least one expected file appears among first K unique paths. Coverage reports expected files found in first five paths.",
    "",
    "| Repo | Task | Top-1 | Top-3 | Top-5 | Expected coverage @5 |",
    "| --- | --- | ---: | ---: | ---: | ---: |",
])
for row in rows:
    lines.append(
        f"| {row['name']} | {row['task']} | "
        f"{'yes' if row['hit_at'][1] else 'no'} | "
        f"{'yes' if row['hit_at'][3] else 'no'} | "
        f"{'yes' if row['hit_at'][5] else 'no'} | "
        f"{row['expected_hits_at_5']}/{len(row['expected_paths'])} |"
    )
lines.extend([
    "",
    "Expected files:",
])
for row in rows:
    expected = ", ".join(f"`{path}`" for path in row["expected_paths"])
    lines.append(f"- **{row['name']}**: {expected}")
lines.extend([
    "",
    "## Interpretation",
    "",
    "- These are deterministic context-selection benchmarks, not model-edit benchmarks.",
    "- Results vary by hardware, filesystem cache, repo checkout shape, and git/network state.",
    "- Small packets with retrieval misses are failures, not efficiency wins.",
    "- Hand-authored expected files can be incomplete; review benchmark tasks and targets when pinned repos change.",
    "- ScoutPack does not run project commands during measurement.",
])
results_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(results_path)
PY
