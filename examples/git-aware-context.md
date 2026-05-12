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

Sample output:

```md
# ScoutPack Context

Task:
review my PR

Relevant Files:
- `src/context.rs`: function `build_context_packet`
- `src/git.rs`: function `recent_changes`

Recent Changes:
- Range: `refs/heads/main`..`HEAD`
- Summary: 2 files changed, +84 -3
- `src/context.rs`: modified, +42 -1
- `src/git.rs`: added, +42 -0

Current Repo Signals:
- Framework: Unknown from index

Relevant Snippets:
...
```

ScoutPack shows file paths and line counts only. It does not include full diff bodies unless those changed files are selected as normal indexed snippets under the token budget.
