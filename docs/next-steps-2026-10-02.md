# ScoutPack: next investment decision

Reviewed October 2, 2026. This note refines the [active milestones](milestones.md); it does not restart the feature backlog.

## Market evidence and interpretation

[Claude Code offers managed PR review and local diff review](https://code.claude.com/docs/en/code-review). [Cursor Bugbot supports MCP tools for additional review context](https://cursor.com/changelog/04-08-26). These are vendor capability descriptions, not independent quality benchmarks.

Inference: another general reviewer or generic repo-search interface is a weak use of maintainer time. A narrower opportunity is local, inspectable evidence that reviewers can trace to specific commits. MCP makes that evidence accessible within existing workflows; it does not establish demand by itself.

The [MCP tool specification](https://modelcontextprotocol.io/specification/2025-11-25/server/tools) defines discoverable input schemas and structured results, with human control and validation. ScoutPack should expose explicit scope, clear provenance, and honest omissions rather than asking clients to infer them from vague descriptions.

## Product boundary

ScoutPack prepares bounded repository evidence for an existing agent or human reviewer. It does not review autonomously, execute suggested commands, or guarantee selected context is sufficient. Token counts are heuristic estimates, not model-token guarantees. Commit-pinned content is not yet fully deterministic replay.

First audience hypothesis: maintainers who need the same committed review starting point across tools, or want to inspect what code their agent receives. Validate with actual tasks before expanding scope.

## Implemented first

MCP `context` now accepts CLI-equivalent Git scopes. Previously committed review packets were unavailable through that tool. The implementation reuses the snapshot builder, avoids dirty working-tree refresh, validates conflicting scopes, and preserves ordinary working-tree queries. Tests exercise real stdio requests and tool discovery; they do not prove compatibility with every client.

## Ordered work

1. Finish M12 correctness: deterministic ranking/replay tests and canonical-root, symlink, regular-file boundary tests. Keep claims qualified until checks pass.
2. Finish M13 evidence: versioned file/range/hash/reason/omission records, deletion/rename coverage, and affected-package commands. Preserve the existing JSON wrapper; introduce evidence additively.
3. Make one workflow usable outside this checkout: verified release artifacts and checksums, clean install, one Codex and one Claude Code smoke test. Avoid adding more installation options before one works end to end.
4. Recruit five consenting maintainers for two weeks of real PR tasks. Record first-run friction, repeated use, irrelevant selections, omitted dependencies, and failures. Do not collect source or telemetry automatically.
5. Publish a small paired comparison against native agent review with fixed repository commits, tasks, model settings, and a cost cap. Measure accepted findings, missed issues, total usage, and elapsed time. Smaller packets alone do not demonstrate better reviews.
6. Invest in discoverability after a reproducible case exists: runnable example, client-specific setup, relevant integration catalogs, and contributor issues drawn from real failures.

One scoped PR at a time. Each change needs formatting, strict Clippy, tests, docs, changelog, and CI. Defer semantic ranking expansion, federation, hosted services, and rebranding during validation.

## Continue or reduce scope

After six weeks, inspect behavior rather than stars. Proposed signal: three of five pilot maintainers return for at least three tasks over two weeks without reminders. This is a decision threshold, not an achieved result or statistical proof. If users find native workflows sufficient, keep ScoutPack as a small maintained CLI/library instead of inventing features to defend the original pitch.
