# Demo

This demo shows ScoutPack as a context preflight step before asking an AI coding agent to review a branch.

Goal:

```txt
Review a login redirect PR without making the agent inspect the whole repo.
```

## Setup

Use any local repo. The example below uses a small Next.js-style auth fixture.

```bash
scoutpack init
scoutpack pack .
scoutpack context "review login redirect PR" --branch --expand-calls 1 --budget 2200
```

## Output

ScoutPack returns a packet focused on changed files, called helpers, likely commands, and risks.

````md
# ScoutPack Context

Task:
review login redirect PR

Relevant Files:
- `src/middleware/auth.ts`: function `requireAuth`
- `src/lib/session.ts`: function `getSession`; call graph: `requireAuth` calls `getSession` at src/middleware/auth.ts:4

Recent Changes:
- Range: `refs/heads/main`..`HEAD`
- Summary: 1 files changed, +4 -1
- `src/middleware/auth.ts`: modified, +4 -1

Current Repo Signals:
- Framework: Next.js, React, Vitest
- test command: `vitest`
- lint command: `eslint .`
- build command: `next build`

Likely Edit Areas:
- `src/lib/session.ts:5-7`
- `src/middleware/auth.ts:3-18`
- auth/session boundary files
- redirect and destination parameter handling

Relevant Snippets:

```file:src/middleware/auth.ts:3-18
export function requireAuth(request: Request) {
  const session = getSession(request);
  const url = new URL(request.url);

  if (!session && url.pathname !== "/login") {
    url.pathname = "/login";
    url.searchParams.set("next", request.url);
    return Response.redirect(url);
  }

  if (url.pathname === "/login" && url.searchParams.get("next") === request.url) {
    url.searchParams.delete("next");
  }

  return null;
}
```

```file:src/lib/session.ts:5-7
export function getSession(_request?: Request): Session | null {
  return null;
}
```

Commands:
- `vitest`
- `eslint .`
- `next build`

Risks:
- redirect loop if post-login destination points back to login or auth guard (source: `src/middleware/auth.ts:3-18`)
- SSR/client mismatch if session state is checked only client-side (source: `src/middleware/auth.ts:3-18`, `src/lib/session.ts:5-7`)

Token Budget Summary:
- Target budget: 2200 tokens
- ScoutPack keeps required files, commands, and risks before optional snippets.
````

## Agent Handoff

Use the packet as the first message or supporting context:

```txt
Use this ScoutPack context packet to review the login redirect PR.
Focus on correctness, redirect-loop risk, session boundary assumptions, and missing tests.
Do not edit files not listed unless you explain why.
```

## What This Demonstrates

- Branch changes are detected locally through git.
- Called helper `getSession` is included even though only middleware changed.
- Test/lint/build commands are discovered but not executed.
- Risks are grounded in selected source ranges.
- Packet stays small enough to paste into any coding agent.

## Recording Note

`vhs`, `asciinema`, and `agg` are not required to use ScoutPack. A GIF can be generated later from this exact flow for README launch assets.
