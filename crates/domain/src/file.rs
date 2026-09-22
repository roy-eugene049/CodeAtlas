use serde::{Deserialize, Serialize};

use crate::ids::{FileId, RepositoryId};
use crate::language::Language;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFile {
    pub id: FileId,
    pub repository_id: RepositoryId,
    pub path: String,
    pub language: Language,
    pub size_bytes: u64,
    pub hash: String,
}

impl SourceFile {
    pub fn new(
        repository_id: RepositoryId,
        path: impl Into<String>,
        language: Language,
        size_bytes: u64,
        hash: impl Into<String>,
    ) -> Self {
        let path = path.into();
        Self {
            id: FileId::stable(repository_id, &path),
            repository_id,
            path,
            language,
            size_bytes,
            hash: hash.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::RepositoryId;

    #[test]
    fn creates_source_file_for_repository() {
        let repo_id = RepositoryId::new();
        let file = SourceFile::new(repo_id, "src/lib.rs", Language::Rust, 128, "hash");
        assert_eq!(file.repository_id, repo_id);
        assert_eq!(file.language, Language::Rust);
        assert_eq!(file.path, "src/lib.rs");
    }
}
