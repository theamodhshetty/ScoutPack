# ScoutPack: product, engineering, and maintenance review

Date: September 25, 2026. Repository inspected at `937670417f0d3f77c72d5ce0da647f2a7f575fb8`.

This is a decision proposal, not a release announcement. Findings distinguish observed behavior, source-level concerns, and unvalidated product hypotheses. Existing roadmap items are not automatically approved by this document.

October 1 implementation update: whole-packet budget fallback and commit-only Git context have been repaired, with regression tests and resolved commit IDs. Branch review now uses merge base. Findings below preserve the September 25 baseline rather than describe all current behavior. Structured evidence, broad deterministic replay, privacy-boundary audit, and demand validation remain open; see [active milestones](milestones.md).

## 1. Decision

Keep ScoutPack, but stop expanding its general-purpose feature list. Fund a six-week, time-boxed validation cycle around **repeatable agent-assisted PR review**. Fix the trust contract before broad promotion.

Proposed positioning:

> ScoutPack prepares reviewable, reproducible code context for agent-assisted PR reviews.

Today, "reproducible" is an engineering objective, not an established guarantee. Until snapshot and deterministic-output work ships, public copy should say: "Local, task-focused code context for agent-assisted review."

The first audience should be OSS maintainers and small engineering teams reviewing the same change through more than one agent or across repeated review sessions. This is a hypothesis to test, not evidence that these users will adopt it. Avoid positioning first for regulated enterprises or enormous monorepos: neither compliance suitability nor that scale has been established.

Do not rename ScoutPack, rebuild its stack, become a coding agent, or launch a hosted service. Retain useful governance and security documents. More branding ceremony is not the current constraint.

## 2. What exists now

ScoutPack is more substantial than the old v0.1 feature list suggests:

- Rust CLI, SQLite/FTS index, tree-sitter parsing for JavaScript/TypeScript, Python, Rust, Go, and Solidity.
- Metadata-aware incremental indexing, automatic refresh, doctor, and watch mode.
- Task ranking, snippets, symbols, import/call expansion, heuristic inspection hints, and suggested project commands.
- Git-scoped context, prompt templates, Markdown/JSON/XML output, and MCP over stdio and HTTP.
- Optional local embedding support behind a feature flag.

Current conceptual pipeline:

```text
task / Git scope
  -> scan files and apply ignore rules
  -> compare cached metadata; hash and parse changed content
  -> SQLite files, symbols, chunks, imports, calls, and FTS
  -> lexical ranking plus optional semantic and relationship expansion
  -> scope candidates, collect commands and inspection hints
  -> assemble Markdown; wrap for JSON/XML or return through MCP
  -> agent decides what to inspect, edit, and test
```

It is not a compiler-grade dependency analyzer, a security scanner, or an immutable repository snapshot. Index freshness and reproducibility are different properties. A fresh index follows the current working tree; a reproducible packet must identify exactly which content was used.

### Adoption and distribution snapshot

GitHub inspection on September 25 showed one star, zero forks, zero open issues, and zero open PRs. Latest push was July 13. Latest public release was `v0.1.0`, dated May 11; that release had **no attached binary assets**. Repository code and release documentation are ahead of the installed release story.

GitHub clone traffic returned 19 clones and 19 unique cloners in the returned recent window. These can include automation and maintainer activity. They are not proof of 19 users, successful activation, or retention.

The README's binary-installer path must be checked against actual published assets. A release workflow that could produce binaries is not an available binary distribution.

## 3. What changed around ScoutPack

The competitive unit is no longer "can this tool fit code into a prompt?" It is "does this improve a complete task compared with the agent's existing retrieval workflow?"

### OpenAI / ChatGPT / Codex

OpenAI's current agent documentation describes managed and application-owned runtimes, tool use, MCP, and context management. Its compaction API supports long-running context management. Those capabilities weaken the case for an external tool whose only benefit is reducing prompt length. They do not establish repository provenance or make a particular selected file set reproducible. ScoutPack should supply evidence through existing runtimes rather than build another runtime. API capabilities should not be assumed to exist identically in every ChatGPT user interface. [Agent runtimes](https://developers.openai.com/api/docs/guides/agents), [compaction](https://developers.openai.com/api/docs/guides/compaction).

### Claude Code

Claude Code skills support progressive loading of instructions and supporting material; subagents provide separate task contexts, including native exploration. A giant always-injected ScoutPack packet would compete with these mechanisms. A small review skill that calls ScoutPack when needed fits better. This does not prove that a skill beats native exploration; test it. [Skills](https://code.claude.com/docs/en/skills), [subagents](https://code.claude.com/docs/en/sub-agents).

### Cursor, Aider, and other clients

Cursor documents built-in code search. Aider already supplies a compact repository map with important symbols. Their users do not need a second index merely because ScoutPack can index code. Gemini CLI supports MCP, making protocol compatibility preferable to another bespoke integration. [Cursor search](https://cursor.com/docs/agent/tools/search), [Aider repository map](https://aider.chat/docs/repomap.html), [Gemini MCP](https://geminicli.com/docs/tools/mcp-server/).

### Frameworks and direct competitors

Deep Agents includes context-management facilities, skills, and subagents. Repomix is not only a whole-repo text dump: it has tree-sitter compression and MCP read/search capabilities. Serena offers symbol-level retrieval and editing backed by language tooling. "AST + MCP" is therefore not a unique differentiator. ScoutPack's possible advantage is a smaller, read-only, portable evidence contract with predictable behavior, but that advantage must be demonstrated. [Deep Agents](https://docs.langchain.com/oss/python/deepagents/overview), [Repomix compression](https://repomix.com/guide/code-compress), [Repomix MCP](https://repomix.com/guide/mcp-server), [Serena](https://github.com/oraios/serena).

### Consequence

Larger context windows, caching, compaction, native search, and isolated subagents do not make retrieval irrelevant. They make token reduction alone a poor product claim. Optimize correctness, review completeness, elapsed time, total billable usage, and repeatability. A larger useful packet can outperform a tiny incomplete packet. Progressive disclosure remains a useful design principle, not a proprietary ScoutPack invention. [Anthropic context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents).

## 4. Engineering findings that change priorities

### 4.1 Budget compliance is not guaranteed

Observed local smoke test:

```bash
./target/debug/scoutpack context "fix login redirect loop" --budget 100 --json --no-refresh
```

It returned `budget: 100` and `estimated_tokens: 662`. First observed with the existing release executable, this was reproduced with the freshly built debug executable after `cargo test --locked`, using the existing local index without refresh. Source inspection independently explains the failure: `src/context.rs` budgets snippets but can return an oversized prefix and suffix after dropping snippets. The fallback is not itself reduced to fit.

The same output mixed Next.js, FastAPI, and Go fixture commands. This repository contains those fixtures, so detection is understandable; presenting all of them as useful commands for one task is not.

Required change: budget every section of the emitted payload, return an explicit error when a minimum useful response cannot fit, and include deterministic omission reasons. Define whether transport wrappers count. Distinguish the present estimator from a model-specific tokenizer; do not promise exact provider token counts from a heuristic.

### 4.2 Determinism and machine-readable evidence need stronger contracts

`src/search.rs` uses score-only ordering in places, including candidates originating in hash-based collections. Equal scores need explicit stable tie-breakers. Scanner and database ordering should also be deliberate.

`src/output.rs` currently wraps a Markdown packet string with a few metadata fields. That is JSON output, but not a typed evidence API. `--no-refresh` freezes refresh behavior, not repository contents or a commit snapshot.

Required change: canonical, versioned packet schema with relative paths, ranges, content hashes, selection reasons, budget accounting, omissions, and source identity. Keep volatile timestamps outside the hashed canonical payload. Add stable ordering and byte-for-byte replay tests. Content hashes prove identity, not correctness or security; signatures can wait.

### 4.3 Git scope is not yet a dependable PR-review foundation

`src/git.rs` compares the selected main branch tree directly with HEAD for branch mode rather than using a merge base. A diverged base can introduce unrelated changes. Snippets come from the current index, so a historical Git range can be paired with current working-tree content.

Required change: explicit PR merge-base semantics, revision-consistent content, and clear handling of dirty files, renames, deleted files, and missing base references. Preserve old CLI behavior through explicit options or a documented deprecation path where semantics change. MCP context requests also need equivalent Git scope rather than requiring a different workflow from the CLI.

### 4.4 Incremental indexing still has whole-repository costs

`src/index.rs` loads cached metadata and walks the tree on refresh; generated artifacts are rewritten. Avoiding file-content reads is valuable but is not O(changed-files) indexing. `src/scanner.rs` does not consistently prune generated directories before descending. Search and call expansion scan broad symbol/edge sets in memory.

Measure default end-to-end latency, including refresh, before optimizing. Prioritize directory pruning, indexed SQL queries, avoiding unchanged artifact writes, and coherent per-root index snapshots. A dirty-file queue can follow if profiling justifies it; watcher overflow and ignore-rule changes must trigger reconciliation.

Concurrent refresh is another inspection concern: per-server mutexes are not a cross-process writer contract. Test multiple CLI/MCP callers, failed writes, and atomic replacement before claiming reliable concurrent use.

### 4.5 Relationship and semantic scores can mislead

Call expansion matches symbol names broadly; identical names in unrelated files can be confused. Treat it as heuristic expansion, not a resolved call graph. Prefer file/scope/symbol identities, local/import resolution, and explicit ambiguity.

The optional hybrid path appears to combine normalized hybrid scores with raw lexical scores for lexical-only candidates. Those scales are not directly comparable. Model and vector loading also add per-query work. Fix and benchmark existing semantics before adding models or vector infrastructure.

### 4.6 Privacy claims require boundary tests

Source inspection identifies missing explicit guarantees around canonical-root containment and regular-file handling. Add adversarial tests for escaping symlinks, symlinked index directories, nonregular files, nested repositories, and concurrent file replacement. These are test gaps, not demonstrated exploits in this review.

The semantic cache check should verify the exact model manifest and enforce offline behavior; an arbitrary existing cache file is not proof that a model is complete. Optional model acquisition must never bypass consent because a cache directory happens to be nonempty.

Keep HTTP local for the pilot. Remote multiuser serving needs authentication, origin/access controls, and a separate threat model. Treat repository content as untrusted data, including comments that contain instructions. Delimit it and preserve provenance; do not claim prompt-injection prevention.

### 4.7 Risk hints need honest naming, not more impressive claims

Existing risk documentation already acknowledges heuristics. Preserve that honesty. Rename output toward "inspection prompts" and show the matching evidence and limitations. A task mentioning a redirect loop does not establish that a particular function contains one. Do not market Solidity suggestions as an audit or inferred confidence as a measured probability.

## 5. Proposed product contract

Build one workflow: a reviewer supplies a task and change scope; ScoutPack returns a bounded, inspectable evidence packet that different clients can reuse.

```text
review task + base/head + explicit dirty-tree policy
  -> verify repository boundary and index health
  -> resolve merge base and exact content identity
  -> rank changed code, related tests, and necessary dependencies
  -> construct typed evidence records
  -> apply whole-packet budget; disclose omissions
  -> canonical JSON + readable Markdown
  -> agent reads bounded additional snippets when needed
  -> human/CI checks evidence identity and review outcome
```

Proposed schema fields, not current output:

```text
schema_version, scoutpack_version, task
source: base, head, merge_base, dirty_policy, content_identity
files[]: path, content_hash, role, ranges, symbols, reasons
relationships[]: source, target, kind, resolution_status
related_tests[], inspection_prompts[], suggested_commands[]
omissions[]: item, reason
budget: requested, estimator, used, wrapper_policy
packet_id: hash of canonical payload
```

Scope suggested commands to the selected package. Commands remain suggestions, never executed by core. Redacted or omitted files must not leak through snippets, summaries, related paths, or exported artifacts. Do not introduce cross-tool activity tracking or persistent user memory by default.

One Rust engine should serve CLI and MCP. Start with documented Claude Code and Codex workflows; add clients only through the same tested interface. Preserve native agent search as a fallback. Do not build five separate agent-specific ranking systems.

## 6. Evidence program

The current real-repo table is useful baseline work, but it measures five small repository/task combinations, with 56-210 indexed files. Its compression ratios compare with broad source dumps, not native agent usage. Expected-file search results do not prove those files survive packet budgeting or supply sufficient context. The recorded results predate the latest automatic-refresh work.

Before publishing new performance claims:

1. Rebuild the tested executable and record its hash, build revision, platform, config, repo commit, task, and exact invocation. Do not silently reuse an older release executable while labeling results with current HEAD.
2. Separate cold indexing, warm unchanged refresh, incremental refresh, retrieval-only latency, and default end-to-end context latency. Report p50/p95 with sample counts.
3. Evaluate at least 30 labeled tasks across approximately ten repositories, with at least ten held-out tasks. Include ambiguous names, related tests, negative matches, branch divergence, and package boundaries.
4. Measure the final emitted packet: expected-file recall, useful ranges, unrelated content, omitted tests, and budget compliance at several budgets. Search top-k alone is insufficient.
5. Then run a capped agent experiment: ten tasks, two conditions (native versus native plus ScoutPack), two client harnesses, and three repetitions, totaling 120 runs if affordable. Use the same model/version, starting tree, instructions, permissions, and settings within each comparison. Randomize order.
6. Report correctness, review findings accepted by humans, test outcomes where applicable, elapsed time, total and cached usage, and cost. Publish losses and variance. Small pilot results remain exploratory, not universal speedup claims.

Any external-agent evaluation belongs in a separately invoked, consent-based harness. It must not add network calls, automatic command execution, or telemetry to ScoutPack runtime. Set a monetary cap before running it; start smaller if necessary.

## 7. Six-week execution proposal

Dates are planning windows, not delivery promises. Cap maintainer effort around 8-12 hours per week, approximately 60 hours total. If correctness work exceeds that allocation, reduce later scope instead of skipping its gates.

| Window | Milestone | Exit condition |
| --- | --- | --- |
| Sep 25-27 | Establish baseline and recruit | Reproducible issue list; five prospective pilot reviewers contacted individually; no new feature expansion |
| Sep 28-Oct 4 | M0: trustworthy output | Whole-packet budget tests, stable tie-breakers, boundary tests, honest claims; exact build/install baseline |
| Oct 5-11 | M1: review evidence | Typed schema, revision identity, merge-base/dirty-tree rules, package-scoped commands; CLI/MCP scope parity |
| Oct 12-18 | M2: usable pilot | Verified release assets and checksums, two-client quickstart, held-out packet evaluation; five installation attempts observed |
| Oct 19-25 | M3: measured value | Repeated real reviews plus capped native-agent comparison; fix failures supported by evidence |
| Oct 26-Nov 5 | M4: distribution and decision | Publish one reproducible case study including losses; continue, narrow again, or enter maintenance-only mode |

Use `0.2` for the repair/release baseline and `0.3` for the review-evidence workflow only if appropriate under the actual compatibility changes. Current public release is still `0.1.0`; do not describe unreleased main features as already published versions. A release candidate is preferable to overstating stability.

Each milestone can contain small PRs, but keep only one feature PR in flight. Every PR needs a failing test or explicit user hypothesis, scoped diff, tests, docs, changelog entry, and a clear rollback path. Push completed work and publish only after its gates pass; no bulk end-of-project release.

### Deliberately deferred

- More embedding models, a vector database, and semantic-search marketing.
- Huge-monorepo claims, new language coverage without user demand, and full LSP/call-graph ambitions.
- Hosted MCP, cross-agent memory, multi-repo federation, and enterprise compliance positioning.
- New GUI, renamed brand, more logos, broad SEO pages, and an expanding preset matrix.
- Cryptographic signing and organization-wide policy engines before basic content identity works.

## 8. Promotion that tests demand

Do not wait for a perfect product to speak to users. Start private discovery now; delay broad claims until budget correctness and installation work.

Recruit two OSS maintainers, two engineers who review with multiple agent tools, and one CI/platform engineer. Ask each to bring one recent difficult PR. Observe their existing workflow first; do not lead with a feature demo. Ask what evidence was missing, what repeated work occurred, and what would justify another installed tool.

Offer an opt-in review packet for their real change. Compare against what their native agent already found. Follow up after one and two weeks. Record consented, minimal observations without collecting private code or adding telemetry.

Useful distribution artifacts:

- One case study with an exact public commit, task, packet, native baseline, accepted finding, timing, and failure cases.
- A tested review-workflow example maintainers can place in their own project. GitHub Actions examples must use least privilege and never run untrusted PR code in a privileged context; packet artifact retention and visibility must be explicit.
- A small skill or MCP setup submitted to a relevant catalog only after it works. The official MCP registry is a discovery channel, not evidence of users: [registry API and ownership model](https://github.com/modelcontextprotocol/registry/blob/main/docs/reference/api/official-registry-api.md).
- A candid "when not to use ScoutPack" comparison: native search for simple tasks; Aider for an integrated map; Serena for precise symbol navigation; Repomix for broad packing/compression.

Avoid unsolicited promotional issues, fabricated testimonials, and "100x more efficient" claims derived from dumping an entire repository. Keep ScoutPack's name; make the description and first usable example sharper.

## 9. Maintenance flow

```text
user problem or regression
  -> reproduce and classify
  -> attach fixture, expected behavior, and success metric
  -> choose smallest fix
  -> unit + integration + privacy/budget regression tests
  -> format + strict Clippy + default-feature tests
  -> relevant cross-platform / optional-feature checks
  -> review diff, docs, changelog, and compatibility
  -> merge and push
  -> tagged release with verified artifacts when warranted
  -> confirm installation and request user follow-up
```

Weekly: two short issue-triage sessions, one focused implementation block, and one pilot conversation. Prioritize privacy or incorrect evidence, then installation failures, then demonstrated relevance/performance problems. Publish realistic response expectations, not an unsupported support SLA.

Monthly: dependency/security review, toolchain compatibility, two-client MCP smoke tests, release/documentation synchronization, and a check that roadmap work still maps to repeated user needs. Keep default CI deterministic and offline after dependencies are available. Test optional semantic code separately without silently fetching models.

Release gates: formatting, strict Clippy, tests, final-payload budgets, stable ordering, root-boundary privacy fixtures, Git snapshot behavior, CLI/MCP schema compatibility, and executable installation checks. Test checksums and real downloads, not installer `--help` alone. Claim platform support only where exercised.

Collect aggregate pilot notes manually with consent: activation time, completed task, repeated use, failure reason, and maintainer support time. Stars and download totals are secondary signals, not the success criterion.

## 10. Continue / stop criteria

Proposed decision targets, not measured results:

- Five external users complete installation and one real review.
- Three return for at least three real tasks over two weeks.
- No known packet-budget or tested privacy-boundary failures remain.
- Paired trials show a meaningful benefit, such as roughly 15-20% lower median cost or elapsed time without unacceptable correctness loss, with sample size and uncertainty disclosed; alternatively, users explicitly demonstrate a repeatability/auditability workflow worth keeping even without speed gains.
- Supporting those users fits the stated maintainer time cap.

If installation fails, demand remains untested: fix activation before judging the product. If people can use it but do not return, more grammars and a new logo are unlikely to help. Ask why, test one narrower adjustment, and stop expansion if the answer remains weak.

At the November 5 checkpoint, choose one outcome: continue the proven review workflow; retain a smaller reusable library/evaluation tool if that is the actual demand; or maintain the stable CLI with security fixes and archive speculative milestones. Do not delete useful code merely because growth is slow, and do not justify endless work by sunk cost.

## Bottom line

ScoutPack has enough implementation to test a real workflow. Its immediate shortage is not features: it is a dependable evidence contract, a verified installation path, and independent proof of usefulness beyond native agent search. Repair those, run a bounded pilot, and let repeated use decide what deserves another release.

## Verification performed

- `cargo test --locked`: passed, 33 unit tests and 8 integration tests, on this checkout.
- Existing release and freshly built debug executables both reproduced the small-budget overflow above.
- GitHub repository/release metadata and official ecosystem documentation were inspected on the review date.
- No new benchmark trial, external-agent comparison, optional semantic-feature test, or security exploit validation was performed. Passing existing tests does not cover the missing invariants identified here.
- Only this analysis document was added; no product behavior, release, or remote repository state was changed.
