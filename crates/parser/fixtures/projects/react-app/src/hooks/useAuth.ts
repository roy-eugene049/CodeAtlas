export function useAuth() {
  return { user: "ada", login };
}

export function login(user: string) {
  return user.length > 0;
}
