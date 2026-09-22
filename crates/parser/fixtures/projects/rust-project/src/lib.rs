pub struct AuthService;

impl AuthService {
    pub fn login(&self, user: &str) -> bool {
        !user.is_empty()
    }
}

pub fn bootstrap() -> AuthService {
    let service = AuthService;
    service.login("ada");
    service
}
