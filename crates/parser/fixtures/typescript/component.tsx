import { useAuth } from "./hooks";

export function useSession() {
  return useAuth();
}

export function AuthPage() {
  const session = useAuth();
  return <section>{session ? "signed in" : "guest"}</section>;
}
