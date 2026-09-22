class AuthService:
    def login(self, user: str) -> bool:
        return bool(user)


def bootstrap() -> AuthService:
    service = AuthService()
    service.login("ada")
    return service
