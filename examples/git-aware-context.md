# Git-Aware Context

Use git-aware context when an agent should focus on branch or PR changes instead of the whole repo.

```bash
scoutpack pack .
scoutpack context "review my PR" --branch --budget 3000
```

Other local ranges:

```bash
scoutpack context "review auth changes" --diff main..HEAD
scoutpack context "continue indexing work" --since HEAD~5
```

Run Git-scoped context from the repository root. `--branch` selects changes between the merge base of the detected main branch and HEAD. Explicit `--diff base..head` retains two-endpoint tree comparison; it does not imply a merge-base comparison. `--since` boosts changed files while reading all candidates from HEAD.

All snippets, symbols, imports, call expansion, and project commands come from the resolved content commit, not the current working-tree index. Full base/content commit IDs appear in `Source Revision`. Staged, unstaged, and untracked edits are excluded, even when the requested head is historical or a file was later deleted. Git summaries can mention deleted or skipped paths, but no content from those paths bypasses scanner policy.

Renames are detected by content similarity (50% threshold), independently of the local `diff.renames` setting. A detected move counts as one changed file and is shown as `` `old/path.ts` -> `new/path.ts`: renamed ``. Added/deleted line counts describe edits within the moved file, rather than treating its entire contents as deleted and added. Unrelated additions and deletions remain separate changes. This is similarity-based detection, not a record of which Git command created the commit.

The MCP `recent_changes` result keeps `path` as the destination for a rename and adds `previous_path` with its original location. Other change records omit `previous_path`. Git-scoped packets select snippets from the destination at the requested content commit; deleted paths appear only in the change summary.

ScoutPack builds and deletes a temporary index for each Git-scoped packet. It skips symlinks, submodules, sensitive patterns, oversized blobs, and unsupported files; it honors committed ignore rules plus current root ignore policy. Current root size limits also bound snapshot materialization. Policy/config changes can affect selection, so commit identity does not by itself guarantee byte-for-byte packet reproducibility. Snapshot rebuilding adds latency on larger repositories.

`--no-refresh` does not disable snapshot construction. `--semantic` is rejected with Git scope because the temporary index has no embeddings; omit it rather than use working-tree vectors against a different revision.

Git-scoped CLI templates follow the same rule as context packets: they build a temporary index for the requested commit and leave the working-tree index untouched. They do not need `--no-refresh` to avoid creating or refreshing `.scoutpack/`. Invalid scope combinations and malformed ranges fail before indexing. Templates without a Git scope still refresh the working-tree index by default.

Successful packets fit the complete Markdown payload budget under ScoutPack's estimator. JSON/XML wrappers and template instructions are excluded. Small budgets may omit files or snippets; budgets below the minimum task/revision/section summary return an error instead of oversized output.

## Package guidance

Commands and framework signals in Git-scoped packets come from configuration
directories that contain changed paths, including ancestor workspace/root
configuration. Deleted paths still identify an affected directory; a detected
rename includes both original and destination directories. Configuration and
commands are read from the requested content commit, not dirty working-tree files.

Without a Git scope, guidance follows selected result paths, including import
or call neighbors. If no paths are selected or affected, ScoutPack reports unknown
guidance instead of borrowing commands from arbitrary packages. This is a
directory-ancestry heuristic, not package-manager or dependency-graph resolution.
Ancestor root signals can still describe shared workspace configuration.

Each suggested command includes its configuration source and working directory:

```text
- test command: `vitest run` (from `apps/api/package.json`; run in `apps/api`)
```

ScoutPack only reports commands; it never executes them. Treat command text as
untrusted repository data and review it before running anything. The MCP
`commands` tool remains the whole-repository catalog; packet/template guidance
is the scoped view. Framework sampling applies its 25-chunk cap after scoping,
so unrelated fixtures cannot hide a selected package's signals. Existing packet
budget and omission rules still apply.

Sample output:

```md
# ScoutPack Context

Task:
review my PR

Source Revision:
- Base commit: `<resolved merge-base commit ID>`
- Content commit: `<resolved HEAD commit ID>`
- Committed content only; staged, unstaged, and untracked files excluded.

Relevant Files:
- `src/context.rs`: function `build_context_packet`
- `src/git.rs`: function `recent_changes`

Recent Changes:
- Range: `<resolved merge-base commit ID>`..`<resolved HEAD commit ID>`
- Summary: 2 files changed, +84 -3
- `src/context.rs`: modified, +42 -1
- `src/git.rs`: added, +42 -0

Current Repo Signals:
- Framework: Unknown from index

Relevant Snippets:
...
```

ScoutPack shows file paths and line counts only. It does not include full diff bodies unless those changed files are selected as normal indexed snippets under the token budget.
