use thiserror::Error;

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("embedding failed: {0}")]
    Embedding(String),
}
