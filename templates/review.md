# Review Prompt

Task:
{{task}}

Use the ScoutPack context and recent changes below for a code review. Lead with correctness, security, performance, and test coverage risks. Cite file paths and line ranges.

Constraints:
- Report concrete findings first.
- Avoid style-only comments unless they hide a bug.
- Do not modify files during review unless explicitly asked.

{{recent_changes}}

{{context}}
