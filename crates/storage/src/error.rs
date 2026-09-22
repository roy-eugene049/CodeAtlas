use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(String),
    #[error("invalid stored language: {0}")]
    Language(String),
    #[error("invalid stored symbol kind: {0}")]
    SymbolKind(String),
    #[error("invalid stored relationship kind: {0}")]
    RelationshipKind(String),
    #[error("invalid stored id: {0}")]
    Id(String),
    #[error("invalid stored evolution: {0}")]
    Evolution(String),
    #[error("invalid stored analysis run: {0}")]
    Run(String),
}

impl From<rusqlite::Error> for StorageError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error.to_string())
    }
}

impl From<postgres::Error> for StorageError {
    fn from(error: postgres::Error) -> Self {
        Self::Database(error.to_string())
    }
}
