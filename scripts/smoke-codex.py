#!/usr/bin/env python3
"""Opt-in Codex/MCP smoke check using only a generated repository."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import secrets
import shutil
import signal
import subprocess
import sys
import tempfile


BUDGET = 2500
TASK = "review login rename"
TOOLS = ("context", "recent_changes")
PROMPT = (
    'Use only ScoutPack MCP tools. Call context with task "review login rename", '
    "branch true, and budget 2500. Also call recent_changes with branch true. "
    "Then report the exact string returned by the committed login function, "
    "the original and destination paths of its rename, and the full content commit ID. "
    "Call each tool once. Do not read files, run commands, or edit anything. "
    "Do not guess if a tool fails."
)


def validate_events(events, expected):
    """Require tool evidence; a successful exit or an agent's claim is insufficient."""
    payloads = {}
    answer = ""
    usage = None
    for event in events:
        if not isinstance(event, dict):
            raise ValueError("Unexpected Codex JSONL event shape")
        if event.get("type") in ("error", "turn.failed"):
            raise ValueError("Codex reported an error or failed turn")
        if event.get("type") == "turn.completed":
            usage = event.get("usage", {})
        item = event.get("item", {})
        if not isinstance(item, dict):
            raise ValueError("Unexpected Codex JSONL item shape")
        if item.get("type") in ("command_execution", "file_change", "web_search"):
            raise ValueError("Codex used a tool outside the ScoutPack-only check")
        if event.get("type") != "item.completed":
            continue
        if item.get("type") == "agent_message":
            answer = item.get("text", "")
        if item.get("type") != "mcp_tool_call":
            continue
        tool = item.get("tool")
        if item.get("server") != "scoutpack" or tool not in TOOLS:
            raise ValueError("Unexpected MCP server or tool")
        if tool in payloads:
            raise ValueError("Unexpected repeated MCP call")
        result = item.get("result") or {}
        if not isinstance(result, dict):
            raise ValueError("Unexpected MCP result shape")
        if item.get("status") != "completed" or item.get("error") or result.get("is_error") or result.get("isError"):
            raise ValueError(f"ScoutPack {tool} did not succeed: {item.get('error')}")
        arguments = item.get("arguments") or {}
        if not isinstance(arguments, dict):
            raise ValueError("Unexpected MCP arguments shape")
        if arguments.get("branch") is not True or any(k in arguments for k in ("diff", "since")):
            raise ValueError(f"ScoutPack {tool} did not use the requested branch scope")
        if tool == "context" and (arguments.get("budget") != BUDGET or arguments.get("task") != TASK):
            raise ValueError("Context used an unexpected task or budget")
        payload = result.get("structured_content")
        if not isinstance(payload, dict):
            raise ValueError("Missing structured MCP result; check the Codex JSONL format")
        payloads[tool] = payload
    if usage is None or set(payloads) != set(TOOLS):
        raise ValueError("Missing completed turn or successful ScoutPack calls")

    context = payloads["context"]
    packet = context.get("packet", "")
    required = (expected["marker"], f"Content commit: `{expected['head']}`", "```file:after.ts:")
    if not isinstance(packet, str) or not all(value in packet for value in required):
        raise ValueError("Context omitted committed content, its revision, or the destination snippet")
    if expected["dirty_marker"] in packet:
        raise ValueError("Context included dirty working-tree content")
    estimate = context.get("estimated_tokens")
    if type(estimate) is not int or not 0 < estimate <= BUDGET or context.get("budget") != BUDGET:
        raise ValueError("Context reported an invalid estimated budget")
    recent = payloads["recent_changes"].get("recent_changes", {})
    if not isinstance(recent, dict):
        raise ValueError("Unexpected change summary shape")
    changes = recent.get("changes", [])
    if recent.get("head_commit") != expected["head"] or recent.get("files_changed") != 1:
        raise ValueError("Change summary used the wrong revision or file count")
    if not isinstance(changes, list) or len(changes) != 1 or not isinstance(changes[0], dict) or any(changes[0].get(k) != v for k, v in {
        "path": "after.ts", "previous_path": "before.ts", "status": "renamed",
        "additions": 0, "deletions": 0,
    }.items()):
        raise ValueError("Change summary lost the rename relationship")
    if not isinstance(answer, str) or not all(value in answer for value in (expected["marker"], expected["head"], "before.ts", "after.ts")):
        raise ValueError("Codex's final answer did not use the returned evidence")
    if expected["dirty_marker"] in answer:
        raise ValueError("Codex's final answer used dirty content")
    return {"status": "passed", "content_commit": expected["head"], "tool_calls": list(TOOLS),
            "packet_estimated_tokens": estimate, "codex_usage": usage, "answer": answer}


def make_fixture(parent):
    root = parent / "repo"
    root.mkdir()
    hooks = parent / "empty-hooks"
    hooks.mkdir()
    # Do not run user Git hooks, templates or signing commands in the generated repo.
    environment = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}

    def git(*args):
        return subprocess.check_output([
            "git", "-c", f"core.hooksPath={hooks}", "-c", f"init.templateDir={hooks}",
            "-c", "commit.gpgSign=false", *args,
        ], cwd=root, env=environment, stderr=subprocess.PIPE, text=True).strip()

    git("init", "-b", "main")
    git("config", "user.name", "ScoutPack Smoke")
    git("config", "user.email", "smoke@example.com")
    marker = "COMMITTED_" + secrets.token_hex(8)
    dirty = "DIRTY_" + secrets.token_hex(8)
    (root / "before.ts").write_text(f"export function login() {{ return '{marker}'; }}\n")
    git("add", ".")
    git("commit", "-m", "base")
    git("checkout", "-b", "feature")
    git("mv", "before.ts", "after.ts")
    git("commit", "-m", "rename login")
    head = git("rev-parse", "HEAD")
    (root / "after.ts").write_text(f"export function login() {{ return '{dirty}'; }}\n")
    return root, {"head": head, "marker": marker, "dirty_marker": dirty}


def codex_command(codex, scoutpack, root, model):
    command = [codex, "exec", "--ignore-user-config", "--ephemeral", "--sandbox", "read-only",
               "--json", "-C", str(root)]
    if model:
        command += ["--model", model]
    settings = {"command": scoutpack, "args": ["mcp", str(root)], "required": True,
                "enabled_tools": list(TOOLS)}
    for key, value in settings.items():
        command += ["-c", f"mcp_servers.scoutpack.{key}={json.dumps(value)}"]
    for tool in TOOLS:
        command += ["-c", f'mcp_servers.scoutpack.tools.{tool}.approval_mode="approve"']
    return command + [PROMPT]


def run_codex(command, timeout):
    process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True, start_new_session=True)
    try:
        stdout, stderr = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        # Include the spawned MCP server, not just the Codex parent process.
        os.killpg(process.pid, signal.SIGKILL)
        process.communicate()
        raise ValueError(f"Codex smoke check exceeded {timeout} seconds") from None
    return process.returncode, stdout, stderr


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", action="store_true", help="Opt in to a live Codex call using saved auth/quota")
    parser.add_argument("--scoutpack", default="scoutpack", help="Installed ScoutPack executable")
    parser.add_argument("--codex", default="codex", help="Installed Codex CLI executable")
    parser.add_argument("--model", help="Model override; otherwise use the isolated CLI default")
    parser.add_argument("--timeout", type=int, default=120, help="Live-run timeout in seconds")
    parser.add_argument("--evidence-dir", type=Path, help="New directory for local logs and summary")
    args = parser.parse_args()
    if not args.run:
        parser.error("Pass --run to use saved Codex authentication and model quota")
    if os.name != "posix" or args.timeout <= 0:
        parser.error("This smoke runner requires macOS/Linux and a positive timeout")
    for name in (args.scoutpack, args.codex, "git"):
        if not shutil.which(name):
            parser.error(f"Executable not found: {name}")
    scoutpack = str(Path(shutil.which(args.scoutpack)).resolve())
    codex = str(Path(shutil.which(args.codex)).resolve())
    version = subprocess.check_output([codex, "--version"], text=True, timeout=10).strip()
    help_text = subprocess.check_output([codex, "exec", "--help"], text=True, timeout=10)
    if "--ignore-user-config" not in help_text or "--ephemeral" not in help_text:
        parser.error("Update Codex CLI: --ignore-user-config and --ephemeral are required")
    if args.evidence_dir:
        args.evidence_dir.mkdir(parents=True, exist_ok=False)
    with tempfile.TemporaryDirectory(prefix="scoutpack-codex-") as directory:
        root, expected = make_fixture(Path(directory))
        before = (root / "after.ts").read_bytes()
        code, stdout, stderr = run_codex(codex_command(codex, scoutpack, root, args.model), args.timeout)
        if args.evidence_dir:
            (args.evidence_dir / "events.jsonl").write_text(stdout)
            (args.evidence_dir / "stderr.log").write_text(stderr)
        if code != 0:
            raise ValueError(f"Codex exited with status {code}; check authentication and MCP startup")
        report = validate_events([json.loads(line) for line in stdout.splitlines() if line.strip()], expected)
        if {p.name for p in root.iterdir()} != {".git", "after.ts"} or (root / "after.ts").read_bytes() != before:
            raise ValueError("The smoke run changed the fixture or created a working-tree index")
        report.update({"codex_version": version, "requested_model": args.model,
                       "scoutpack_sha256": hashlib.sha256(Path(scoutpack).read_bytes()).hexdigest(),
                       "events_sha256": hashlib.sha256(stdout.encode()).hexdigest()})
    summary = json.dumps(report, indent=2) + "\n"
    if args.evidence_dir:
        (args.evidence_dir / "summary.json").write_text(summary)
    print(summary, end="")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(f"Codex smoke check failed: {error}", file=sys.stderr)
        sys.exit(1)
