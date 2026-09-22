import { AuthService } from "../services/auth";

export function useAuth(service: AuthService) {
  return service.login("ada");
}
