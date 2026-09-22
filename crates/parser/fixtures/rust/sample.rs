use crate::db::UserRepository;

pub struct AuthService {
    repo: UserRepository,
}

pub trait Repository {
    fn find_user(&self, name: &str) -> Option<String>;
}

impl AuthService {
    pub fn login(&self, user: &str) -> bool {
        self.repo.find_user(user).is_some()
    }
}

impl Repository for AuthService {
    fn find_user(&self, name: &str) -> Option<String> {
        Some(name.to_string())
    }
}

pub fn bootstrap() {
    let service = AuthService {
        repo: UserRepository,
    };
    service.login("ada");
}
