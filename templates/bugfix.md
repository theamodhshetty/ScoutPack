# Bugfix Prompt

Task:
{{task}}

Use the ScoutPack context below to find the root cause, propose the smallest safe fix, and add or update a regression test where practical.

Constraints:
- Stay local to the likely files unless evidence points elsewhere.
- Preserve existing behavior outside the bug.
- Do not run project commands unless the user explicitly asks.

{{recent_changes}}

{{context}}
