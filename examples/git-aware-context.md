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

ScoutPack builds and deletes a temporary index for each Git-scoped packet. It skips symlinks, submodules, sensitive patterns, oversized blobs, and unsupported files; it honors committed ignore rules plus current root ignore policy. Current root size limits also bound snapshot materialization. Policy/config changes can affect selection, so commit identity does not by itself guarantee byte-for-byte packet reproducibility. Snapshot rebuilding adds latency on larger repositories.

`--no-refresh` does not disable snapshot construction. `--semantic` is rejected with Git scope because the temporary index has no embeddings; omit it rather than use working-tree vectors against a different revision.

Successful packets fit the complete Markdown payload budget under ScoutPack's estimator. JSON/XML wrappers and template instructions are excluded. Small budgets may omit files or snippets; budgets below the minimum task/revision/section summary return an error instead of oversized output.

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
