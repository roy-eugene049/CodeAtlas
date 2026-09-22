use serde::{Deserialize, Serialize};

use crate::sensitive::is_sensitive_path;

/// One commit in a repository's evolution layer. No git command objects here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitSummary {
    pub sha: String,
    pub author_name: String,
    pub author_email: String,
    pub authored_at: i64,
    pub subject: String,
    pub files_changed: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchSummary {
    pub name: String,
    pub sha: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorSummary {
    pub name: String,
    pub email: String,
    pub commit_count: u64,
    pub file_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileHotspot {
    pub path: String,
    pub change_count: u64,
    pub contributor_count: u64,
    pub last_commit_sha: String,
    pub last_authored_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitTouch {
    pub sha: String,
    pub author_name: String,
    pub author_email: String,
    pub authored_at: i64,
    pub subject: String,
    pub paths: Vec<String>,
}

/// Derived git history. Domain-owned; collectors live in the git crate.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitEvolution {
    pub commits: Vec<CommitSummary>,
    pub branches: Vec<BranchSummary>,
    pub authors: Vec<AuthorSummary>,
    pub hotspots: Vec<FileHotspot>,
}

impl GitEvolution {
    pub fn assemble(touches: Vec<CommitTouch>, branches: Vec<BranchSummary>) -> Self {
        let commits = touches
            .iter()
            .map(|touch| CommitSummary {
                sha: touch.sha.clone(),
                author_name: touch.author_name.clone(),
                author_email: touch.author_email.clone(),
                authored_at: touch.authored_at,
                subject: touch.subject.clone(),
                files_changed: touch.paths.len() as u32,
            })
            .collect();

        Self {
            authors: authors_from(&touches),
            hotspots: hotspots_from(&touches),
            commits,
            branches,
        }
    }
}

fn authors_from(touches: &[CommitTouch]) -> Vec<AuthorSummary> {
    let mut by_email: std::collections::BTreeMap<String, AuthorSummary> =
        std::collections::BTreeMap::new();
    for touch in touches {
        let entry = by_email
            .entry(touch.author_email.to_ascii_lowercase())
            .or_insert_with(|| AuthorSummary {
                name: touch.author_name.clone(),
                email: touch.author_email.clone(),
                commit_count: 0,
                file_count: 0,
            });
        entry.commit_count += 1;
        entry.file_count += touch.paths.len() as u64;
        if entry.name.is_empty() {
            entry.name = touch.author_name.clone();
        }
    }
    let mut authors = by_email.into_values().collect::<Vec<_>>();
    authors.sort_by(|a, b| b.commit_count.cmp(&a.commit_count).then_with(|| a.name.cmp(&b.name)));
    authors
}

fn hotspots_from(touches: &[CommitTouch]) -> Vec<FileHotspot> {
    let mut by_path: std::collections::BTreeMap<String, FileHotspot> =
        std::collections::BTreeMap::new();
    let mut contributors: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
        std::collections::BTreeMap::new();

    for touch in touches {
        for path in &touch.paths {
            if is_sensitive_path(path) {
                continue;
            }
            let entry = by_path.entry(path.clone()).or_insert_with(|| FileHotspot {
                path: path.clone(),
                change_count: 0,
                contributor_count: 0,
                last_commit_sha: touch.sha.clone(),
                last_authored_at: touch.authored_at,
            });
            entry.change_count += 1;
            if touch.authored_at >= entry.last_authored_at {
                entry.last_authored_at = touch.authored_at;
                entry.last_commit_sha = touch.sha.clone();
            }
            contributors
                .entry(path.clone())
                .or_default()
                .insert(touch.author_email.to_ascii_lowercase());
        }
    }

    let mut hotspots = by_path.into_values().collect::<Vec<_>>();
    for hotspot in &mut hotspots {
        hotspot.contributor_count = contributors
            .get(&hotspot.path)
            .map(std::collections::BTreeSet::len)
            .unwrap_or(0) as u64;
    }
    hotspots.sort_by(|a, b| {
        b.change_count
            .cmp(&a.change_count)
            .then_with(|| b.contributor_count.cmp(&a.contributor_count))
            .then_with(|| a.path.cmp(&b.path))
    });
    hotspots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotspot_counts_changes_and_contributors() {
        let evolution = GitEvolution::assemble(
            vec![
                touch("aaa", "Ada", "ada@ex.com", 10, "src/auth/session.ts"),
                touch("bbb", "Ada", "ada@ex.com", 20, "src/auth/session.ts"),
                touch("ccc", "Ben", "ben@ex.com", 30, "src/auth/session.ts"),
                touch("ddd", "Ben", "ben@ex.com", 40, "src/lib.rs"),
            ],
            vec![],
        );
        let session = evolution
            .hotspots
            .iter()
            .find(|hotspot| hotspot.path == "src/auth/session.ts")
            .expect("session hotspot");
        assert_eq!(session.change_count, 3);
        assert_eq!(session.contributor_count, 2);
        assert_eq!(evolution.authors.len(), 2);
        assert_eq!(evolution.authors[0].commit_count, 2);
        assert!(!evolution.hotspots.iter().any(|hotspot| hotspot.path == ".env"));
    }

    #[test]
    fn skips_secret_paths() {
        let evolution = GitEvolution::assemble(
            vec![touch("aaa", "Ada", "ada@ex.com", 1, ".env")],
            vec![],
        );
        assert!(evolution.hotspots.is_empty());
    }

    fn touch(sha: &str, name: &str, email: &str, at: i64, path: &str) -> CommitTouch {
        CommitTouch {
            sha: sha.into(),
            author_name: name.into(),
            author_email: email.into(),
            authored_at: at,
            subject: "wip".into(),
            paths: vec![path.into()],
        }
    }
}
