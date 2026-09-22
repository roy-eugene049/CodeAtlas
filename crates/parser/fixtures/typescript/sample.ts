import { UserRepository } from "./repository";

export interface User {
  id: string;
  name: string;
}

export class AuthService {
  constructor(private readonly repository: UserRepository) {}

  login(user: string): User | null {
    return this.repository.findUser(user);
  }
}

export function bootstrap(repository: UserRepository): AuthService {
  return new AuthService(repository);
}
