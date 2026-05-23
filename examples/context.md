# ScoutPack Context

Task:
fix login redirect loop

Relevant Files:
- `src/app/login/page.tsx`: component `LoginPage`
- `src/lib/session.ts`: function `getSession`
- `src/middleware/auth.ts`: function `requireAuth`

Current Repo Signals:
- Framework: Next.js, React, Vitest
- test command: `vitest`
- lint command: `eslint .`
- build command: `next build`

Likely Edit Areas:
- `src/app/login/page.tsx:3-11`
- `src/lib/session.ts:5-7`
- `src/middleware/auth.ts:3-14`
- auth/session boundary files
- redirect and destination parameter handling

Relevant Snippets:

```file:src/middleware/auth.ts:3-14
export function requireAuth(request: Request) {
  const session = getSession(request);
  const url = new URL(request.url);

  if (!session && url.pathname !== "/login") {
    url.pathname = "/login";
    url.searchParams.set("next", request.url);
    return Response.redirect(url);
  }

  return null;
}
```

Commands:
- `vitest`
- `eslint .`
- `next build`

Risk Hints:
- [medium] redirect loop if post-login destination points back to login or auth guard; inspect routing/session boundary (source: `src/middleware/auth.ts:3-14`)
- [low] SSR/client mismatch if session state is checked only client-side; inspect server/client boundary (source: `src/app/login/page.tsx:3-11`, `src/lib/session.ts:5-7`)
