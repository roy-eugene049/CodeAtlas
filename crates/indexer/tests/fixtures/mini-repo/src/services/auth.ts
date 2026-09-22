import { UserRepository } from "../db/repository";

export class AuthService {
  constructor(private readonly repository: UserRepository) {}

  login(user: string): boolean {
    return this.repository.findUser(user);
  }
}
