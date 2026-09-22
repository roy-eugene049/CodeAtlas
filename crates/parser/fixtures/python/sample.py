from db.repository import UserRepository

TIMEOUT = 30


class Service:
    pass


class AuthService(Service):
    def __init__(self, repository: UserRepository) -> None:
        self.repository = repository

    def login(self, user: str) -> bool:
        return self.repository.find_user(user) is not None


def bootstrap(repository: UserRepository) -> AuthService:
    service = AuthService(repository)
    service.login("ada")
    return service
