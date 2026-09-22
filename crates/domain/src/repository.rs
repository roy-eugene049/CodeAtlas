use serde::{Deserialize, Serialize};

use crate::ids::RepositoryId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repository {
    pub id: RepositoryId,
    pub name: String,
    pub url: String,
    pub default_branch: String,
    pub commit_sha: String,
}

impl Repository {
    pub fn new(
        name: impl Into<String>,
        url: impl Into<String>,
        default_branch: impl Into<String>,
        commit_sha: impl Into<String>,
    ) -> Self {
        Self {
            id: RepositoryId::new(),
            name: name.into(),
            url: url.into(),
            default_branch: default_branch.into(),
            commit_sha: commit_sha.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_repository_with_identity() {
        let repo = Repository::new("CodeAtlas", "https://github.com/user/project", "main", "abc123");
        assert_eq!(repo.name, "CodeAtlas");
        assert_eq!(repo.default_branch, "main");
        assert_eq!(repo.commit_sha, "abc123");
    }
}
