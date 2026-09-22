use std::collections::HashSet;
use std::path::Path;

use crate::cli::{git_root, stdout};
use crate::error::GitError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepoDiff {
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub deleted: Vec<String>,
    pub renamed: Vec<(String, String)>,
}

impl RepoDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.modified.is_empty()
            && self.deleted.is_empty()
            && self.renamed.is_empty()
    }

    pub fn merge(&mut self, other: RepoDiff) {
        extend_unique(&mut self.added, other.added);
        extend_unique(&mut self.modified, other.modified);
        extend_unique(&mut self.deleted, other.deleted);
        for rename in other.renamed {
            if !self.renamed.contains(&rename) {
                self.renamed.push(rename);
            }
        }
    }

    pub fn parse_paths(&self) -> HashSet<String> {
        let mut paths = HashSet::new();
        paths.extend(self.added.iter().cloned());
        paths.extend(self.modified.iter().cloned());
        paths.extend(self.renamed.iter().map(|(_, new)| new.clone()));
        paths
    }

    pub fn drop_paths(&self) -> HashSet<String> {
        let mut paths = HashSet::new();
        paths.extend(self.deleted.iter().cloned());
        paths.extend(self.renamed.iter().map(|(old, _)| old.clone()));
        paths
    }

    pub fn summary(&self) -> String {
        format!(
            "{} added, {} modified, {} deleted, {} renamed",
            self.added.len(),
            self.modified.len(),
            self.deleted.len(),
            self.renamed.len()
        )
    }
}

/// Files that changed between two commits, plus the working tree if dirty.
pub fn diff_index_range(
    source_root: &Path,
    from_sha: &str,
    to_sha: &str,
) -> Result<RepoDiff, GitError> {
    let git_root = git_root(source_root)?;
    let prefix = source_prefix(source_root, &git_root);
    let mut diff = if looks_like_sha(from_sha) && looks_like_sha(to_sha) && from_sha != to_sha {
        let raw = stdout(
            &git_root,
            &["diff", "--name-status", "--find-renames", from_sha, to_sha],
        )?;
        parse_name_status(&raw, &prefix)
    } else {
        RepoDiff::default()
    };
    diff.merge(working_tree_diff(source_root)?);
    Ok(diff)
}

pub fn working_tree_diff(source_root: &Path) -> Result<RepoDiff, GitError> {
    let git_root = match git_root(source_root) {
        Ok(root) => root,
        Err(_) => return Ok(RepoDiff::default()),
    };
    let prefix = source_prefix(source_root, &git_root);
    let raw = stdout(&git_root, &["status", "--porcelain", "-uall"])?;
    Ok(parse_porcelain(&raw, &prefix))
}

pub fn looks_like_sha(value: &str) -> bool {
    let value = value.trim();
    value.len() >= 7 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

pub fn parse_name_status(raw: &str, prefix: &str) -> RepoDiff {
    let mut diff = RepoDiff::default();
    for line in raw.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        let status = line.chars().next().unwrap_or(' ');
        let rest = line.get(1..).unwrap_or("").trim();
        match status {
            'A' => {
                let path = strip_prefix(rest, prefix);
                if !path.is_empty() {
                    diff.added.push(path);
                }
            }
            'M' | 'T' => {
                let path = strip_prefix(rest, prefix);
                if !path.is_empty() {
                    diff.modified.push(path);
                }
            }
            'D' => {
                let path = strip_prefix(rest, prefix);
                if !path.is_empty() {
                    diff.deleted.push(path);
                }
            }
            'R' | 'C' => {
                let cols = rest
                    .split('\t')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>();
                let (old, new) = match cols.as_slice() {
                    [old, new] => (*old, *new),
                    [score, old, new] if score.chars().all(|ch| ch.is_ascii_digit()) => (*old, *new),
                    _ => continue,
                };
                let old = strip_prefix(old, prefix);
                let new = strip_prefix(new, prefix);
                if !old.is_empty() && !new.is_empty() {
                    if status == 'R' {
                        diff.renamed.push((old, new));
                    } else {
                        diff.added.push(new);
                    }
                }
            }
            _ => {}
        }
    }
    diff
}

pub fn parse_porcelain(raw: &str, prefix: &str) -> RepoDiff {
    let mut diff = RepoDiff::default();
    for line in raw.lines() {
        if line.len() < 4 {
            continue;
        }
        let code = &line[..2];
        let raw_path = line[3..].trim();
        let path = strip_prefix(
            raw_path.rsplit_once(" -> ").map(|(_, new)| new).unwrap_or(raw_path),
            prefix,
        );
        if path.is_empty() {
            continue;
        }
        if code.contains('D') {
            diff.deleted.push(path);
        } else if code.contains('A') || code.contains('?') {
            diff.added.push(path);
        } else if code.contains('R') {
            diff.modified.push(path);
        } else {
            diff.modified.push(path);
        }
    }
    diff
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

fn strip_prefix(path: &str, prefix: &str) -> String {
    let normalized = path.replace('\\', "/").trim().to_string();
    if prefix.is_empty() {
        return normalized;
    }
    let prefix = prefix.trim_start_matches("./").trim_end_matches('/');
    normalized
        .strip_prefix(&format!("{prefix}/"))
        .unwrap_or(&normalized)
        .to_string()
}

fn extend_unique(target: &mut Vec<String>, incoming: Vec<String>) {
    for path in incoming {
        if !target.contains(&path) {
            target.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_added_modified_deleted_renamed() {
        let raw = "A\tsrc/new.ts\nM\tsrc/old.ts\nD\tsrc/gone.ts\nR100\tsrc/a.ts\tsrc/b.ts\n";
        let diff = parse_name_status(raw, "");
        assert_eq!(diff.added, vec!["src/new.ts"]);
        assert_eq!(diff.modified, vec!["src/old.ts"]);
        assert_eq!(diff.deleted, vec!["src/gone.ts"]);
        assert_eq!(diff.renamed, vec![("src/a.ts".into(), "src/b.ts".into())]);
        assert!(diff.parse_paths().contains("src/b.ts"));
        assert!(diff.drop_paths().contains("src/a.ts"));
    }

    #[test]
    fn diffs_a_real_commit_range() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        run(root, &["git", "init", "-b", "main"]);
        run(root, &["git", "config", "user.name", "Ada"]);
        run(root, &["git", "config", "user.email", "ada@ex.com"]);
        write(root, "src/keep.ts", "export const keep = 1;\n");
        write(root, "src/touch.ts", "export const touch = 1;\n");
        write(root, "src/gone.ts", "export const gone = 1;\n");
        write(root, "src/rename.ts", "export const renamed = 1;\n");
        run(root, &["git", "add", "."]);
        run(root, &["git", "commit", "-m", "A"]);
        let sha_a = stdout(root, &["rev-parse", "HEAD"]).expect("sha a");

        std::fs::write(root.join("src/touch.ts"), "export const touch = 2;\n").expect("edit");
        std::fs::remove_file(root.join("src/gone.ts")).expect("delete");
        std::fs::rename(root.join("src/rename.ts"), root.join("src/renamed.ts")).expect("rename");
        write(root, "src/new.ts", "export const created = 1;\n");
        run(root, &["git", "add", "-A"]);
        run(root, &["git", "commit", "-m", "B"]);
        let sha_b = stdout(root, &["rev-parse", "HEAD"]).expect("sha b");

        let diff = diff_index_range(root, &sha_a, &sha_b).expect("diff");
        assert!(diff.added.iter().any(|path| path == "src/new.ts"));
        assert!(diff.modified.iter().any(|path| path == "src/touch.ts") || diff.renamed.iter().any(|(_, new)| new == "src/renamed.ts"));
        assert!(
            diff.deleted.iter().any(|path| path == "src/gone.ts")
                || diff.renamed.iter().any(|(old, _)| old == "src/gone.ts")
        );
        assert!(
            diff.renamed.iter().any(|(old, new)| old == "src/rename.ts" && new == "src/renamed.ts")
                || (diff.deleted.contains(&"src/rename.ts".into())
                    && diff.added.contains(&"src/renamed.ts".into()))
        );
        assert!(!diff.parse_paths().contains("src/keep.ts"));
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
