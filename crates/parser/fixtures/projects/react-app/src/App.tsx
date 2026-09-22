import { useAuth } from "./hooks/useAuth";

export function App() {
  const session = useAuth();
  return <main>{session.user}</main>;
}
