# ScoutPack Context

Task:
fix login redirect loop

Relevant Files:
- `src/middleware/auth.ts`: function `requireAuth`
- `src/app/login/page.tsx`: component `LoginPage`

Current Repo Signals:
- Framework: Next.js, React
- test command: `vitest`
- lint command: `eslint .`

Likely Edit Areas:
- redirect and destination parameter handling
- auth/session boundary files

Risks:
- redirect loop if post-login destination points back to login or auth guard
- SSR/client mismatch if session state is checked only client-side

