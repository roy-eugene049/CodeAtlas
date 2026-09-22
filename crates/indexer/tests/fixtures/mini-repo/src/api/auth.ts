import { AuthService } from "../services/auth";
import { useAuth } from "../hooks/useAuth";

export function AuthController(service: AuthService) {
  return useAuth(service);
}
