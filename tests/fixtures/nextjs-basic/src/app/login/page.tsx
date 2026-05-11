import { getSession } from "../../lib/session";

export function LoginPage() {
  const session = getSession();

  if (session) {
    return <div>Already signed in</div>;
  }

  return <form>Login</form>;
}

