export class UserRepository {
  findUser(name: string): boolean {
    return name.length > 0;
  }
}
