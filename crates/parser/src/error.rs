use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("failed to configure tree-sitter language: {0}")]
    Language(String),
    #[error("tree-sitter failed to produce a syntax tree")]
    ParseFailed,
    #[error("no parser registered for this file")]
    UnsupportedLanguage,
}
