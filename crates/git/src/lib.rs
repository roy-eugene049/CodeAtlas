//! Read-only git history. Never writes to the user's repository.

mod cli;
mod diff;
mod error;

use std::path::Path;

use codeatlas_domain::{BranchSummary, CommitTouch, GitEvolution};

pub use diff::{
    diff_index_range, looks_like_sha, parse_name_status, working_tree_diff, ChangeKind, RepoDiff,
};
pub use error::GitError;

const MAX_COMMITS: usize = 400;

/// Collect commits, branches, authors, and file hotspots under `source_root`.
pub fn collect_evolution(source_root: &Path) -> Result<GitEvolution, GitError> {
    let git_root = match cli::git_root(source_root) {
        Ok(root) => root,
        Err(_) => return Ok(GitEvolution::default()),
    };
    let prefix = source_prefix(source_root, &git_root);
    let log = if prefix.is_empty() {
        cli::stdout(
            &git_root,
            &[
                "log",
                &format!("-{MAX_COMMITS}"),
                "--date=unix",
                "--pretty=format:%H%x1f%an%x1f%ae%x1f%at%x1f%s",
                "--name-only",
                "--no-merges",
            ],
        )?
    } else {
        cli::stdout(
            &git_root,
            &[
                "log",
                &format!("-{MAX_COMMITS}"),
                "--date=unix",
                "--pretty=format:%H%x1f%an%x1f%ae%x1f%at%x1f%s",
                "--name-only",
                "--no-merges",
                "--",
                &prefix,
            ],
        )?
    };
    let touches = parse_log(&log, &prefix);
    let branches = collect_branches(&git_root)?;
    Ok(GitEvolution::assemble(touches, branches))
}

fn source_prefix(source_root: &Path, git_root: &Path) -> String {
    let Ok(source) = source_root.canonicalize() else {
        return String::new();
    };
    source
        .strip_prefix(git_root)
        .ok()
        .map(|prefix| prefix.to_string_lossy().replace('\\', "/"))
        .filter(|prefix| !prefix.is_empty())
        .unwrap_or_default()
}

fn collect_branches(git_root: &Path) -> Result<Vec<BranchSummary>, GitError> {
    let raw = cli::stdout(
        git_root,
        &[
            "for-each-ref",
            "--format=%(refname:short)%1f%(objectname)%1f%(HEAD)",
            "refs/heads",
        ],
    )?;
    Ok(parse_branches(&raw))
}

pub fn parse_log(raw: &str, prefix: &str) -> Vec<CommitTouch> {
    let mut touches = Vec::new();
    let mut current: Option<CommitTouch> = None;
    for line in raw.lines() {
        if line.contains('\u{1f}') {
            if let Some(touch) = current.take() {
                touches.push(touch);
            }
            let Some(parsed) = parse_header(line) else {
                continue;
            };
            current = Some(parsed);
            continue;
        }
        let path = line.trim();
        if path.is_empty() {
            continue;
        }
        let relative = strip_prefix(path, prefix);
        if let Some(touch) = current.as_mut() {
            if !relative.is_empty() {
                touch.paths.push(relative);
            }
        }
    }
    if let Some(touch) = current {
        touches.push(touch);
    }
    touches
}

fn parse_header(line: &str) -> Option<CommitTouch> {
    let parts = line.split('\u{1f}').collect::<Vec<_>>();
    if parts.len() < 5 {
        return None;
    }
    Some(CommitTouch {
        sha: parts[0].to_string(),
        author_name: parts[1].to_string(),
        author_email: parts[2].to_string(),
        authored_at: parts[3].parse().unwrap_or(0),
        subject: parts[4].to_string(),
        paths: Vec::new(),
    })
}

fn parse_branches(raw: &str) -> Vec<BranchSummary> {
    let mut branches = Vec::new();
    for line in raw.lines() {
        let parts = line.split('\u{1f}').collect::<Vec<_>>();
        if parts.len() < 3 {
            continue;
        }
        branches.push(BranchSummary {
            name: parts[0].to_string(),
            sha: parts[1].to_string(),
            is_default: parts[2].trim() == "*",
        });
    }
    branches.sort_by(|a, b| b.is_default.cmp(&a.is_default).then_with(|| a.name.cmp(&b.name)));
    branches
}

fn strip_prefix(path: &str, prefix: &str) -> String {
    let normalized = path.replace('\\', "/");
    if prefix.is_empty() {
        return normalized;
    }
    let prefix = prefix.trim_start_matches("./").trim_end_matches('/');
    normalized
        .strip_prefix(&format!("{prefix}/"))
        .unwrap_or(&normalized)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_log_and_strips_subtree_prefix() {
        let raw = format!(
            "aaa\u{1f}Ada\u{1f}ada@ex.com\u{1f}10\u{1f}fix session\ncrates/app/src/auth/session.ts\n\nbbb\u{1f}Ben\u{1f}ben@ex.com\u{1f}20\u{1f}docs\nREADME.md\n"
        );
        let touches = parse_log(&raw, "crates/app");
        assert_eq!(touches[0].paths, vec!["src/auth/session.ts"]);
        assert_eq!(touches[1].paths, vec!["README.md"]);
    }

    #[test]
    fn collects_hotspots_from_a_temp_repository() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        run(root, &["git", "init", "-b", "main"]);
        run(root, &["git", "config", "user.name", "Ada"]);
        run(root, &["git", "config", "user.email", "ada@ex.com"]);
        write(root, "src/auth/session.ts", "one");
        write(root, "src/lib.rs", "root");
        run(root, &["git", "add", "."]);
        run(root, &["git", "commit", "-m", "init"]);
        write(root, "src/auth/session.ts", "two");
        run(root, &["git", "add", "."]);
        run(root, &["git", "commit", "-m", "session 2"]);
        run(root, &["git", "config", "user.name", "Ben"]);
        run(root, &["git", "config", "user.email", "ben@ex.com"]);
        write(root, "src/auth/session.ts", "three");
        run(root, &["git", "add", "."]);
        run(root, &["git", "commit", "-m", "session 3"]);

        let evolution = collect_evolution(root).expect("collect");
        let session = evolution
            .hotspots
            .iter()
            .find(|hotspot| hotspot.path == "src/auth/session.ts")
            .expect("hotspot");
        assert_eq!(session.change_count, 3);
        assert_eq!(session.contributor_count, 2);
        assert!(evolution.authors.iter().any(|author| author.email == "ada@ex.com"));
        assert!(evolution.branches.iter().any(|branch| branch.name == "main"));
        assert_eq!(evolution.commits.len(), 3);
    }

    fn write(root: &Path, path: &str, body: &str) {
        let full = root.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(full, body).expect("write");
    }

    fn run(cwd: &Path, args: &[&str]) {
        let status = std::process::Command::new(args[0])
            .args(&args[1..])
            .current_dir(cwd)
            .status()
            .expect("spawn");
        assert!(status.success(), "{args:?}");
    }
}
