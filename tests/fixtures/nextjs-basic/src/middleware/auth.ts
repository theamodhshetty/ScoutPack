import { getSession } from "../lib/session";

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

