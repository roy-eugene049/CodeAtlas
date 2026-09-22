use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("embedding failed: {0}")]
    Embedding(String),
    #[error("the language model failed: {0}")]
    Model(String),
    #[error("no retrieved context for this question")]
    EmptyContext,
}
