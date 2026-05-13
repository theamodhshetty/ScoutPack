# Test Prompt

Task:
{{task}}

Use the ScoutPack context below to design focused tests for the relevant code paths. Prefer regression tests for known bugs and behavior tests for public surfaces.

Constraints:
- Reuse existing test patterns and fixtures.
- Keep tests deterministic.
- Do not add broad snapshot churn unless the project already uses it.

{{recent_changes}}

{{context}}
