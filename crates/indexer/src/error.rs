use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("failed to read source: {0}")]
    Io(#[from] std::io::Error),
    #[error("git operation failed: {0}")]
    Git(String),
    #[error("failed to parse {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: codeatlas_parser::ParseError,
    },
    #[error("path is not a readable directory: {0}")]
    InvalidRoot(String),
    #[error("no indexable source files found")]
    EmptyRepository,
    #[error("repository exceeds the file count limit ({0})")]
    TooManyFiles(usize),
    #[error("repository exceeds the size limit ({0} bytes)")]
    RepositoryTooLarge(u64),
}
