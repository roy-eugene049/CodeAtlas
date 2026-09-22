use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("git is not available: {0}")]
    Io(#[from] std::io::Error),
    #[error("git command failed: {0}")]
    Command(String),
    #[error("not a git repository: {0}")]
    NotARepository(String),
}
