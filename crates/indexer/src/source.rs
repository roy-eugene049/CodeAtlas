use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::IndexError;

#[derive(Debug, Clone)]
pub struct MaterializedSource {
    pub root: PathBuf,
    pub name: String,
    pub url: String,
    pub default_branch: String,
    pub commit_sha: String,
}

pub fn materialize_local(path: &Path) -> Result<MaterializedSource, IndexError> {
    let root = path
        .canonicalize()
        .map_err(|_| IndexError::InvalidRoot(path.display().to_string()))?;
    if !root.is_dir() {
        return Err(IndexError::InvalidRoot(root.display().to_string()));
    }

    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repository")
        .to_string();

    let (default_branch, commit_sha) =
        git_identity(&root).unwrap_or_else(|| ("HEAD".to_string(), "working-tree".to_string()));

    Ok(MaterializedSource {
        url: root.to_string_lossy().into_owned(),
        root,
        name,
        default_branch,
        commit_sha,
    })
}

pub fn materialize_git(url: &str, cache_dir: &Path) -> Result<MaterializedSource, IndexError> {
    std::fs::create_dir_all(cache_dir)?;
    let name = repo_name_from_url(url);
    let root = cache_dir.join(&name);

    if root.exists() {
        git_exec(&root, &["fetch", "--all", "--prune", "--depth", "250"])?;
    } else {
        git_exec(cache_dir, &["clone", "--depth", "250", url, &name])?;
    }

    let mut source = materialize_local(&root)?;
    source.url = url.to_string();
    source.name = name;
    Ok(source)
}

pub fn materialize(spec: &str, cache_dir: &Path) -> Result<MaterializedSource, IndexError> {
    let path = Path::new(spec);
    if path.exists() {
        materialize_local(path)
    } else {
        materialize_git(spec, cache_dir)
    }
}

fn git_identity(root: &Path) -> Option<(String, String)> {
    let branch = git_stdout(root, &["rev-parse", "--abbrev-ref", "HEAD"]).ok()?;
    let sha = git_stdout(root, &["rev-parse", "HEAD"]).ok()?;
    Some((branch, sha))
}

fn git_exec(cwd: &Path, args: &[&str]) -> Result<(), IndexError> {
    git_stdout(cwd, args).map(|_| ())
}

fn git_stdout(cwd: &Path, args: &[&str]) -> Result<String, IndexError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| IndexError::Git(error.to_string()))?;
    if !output.status.success() {
        return Err(IndexError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn repo_name_from_url(url: &str) -> String {
    url.trim_end_matches('/')
        .trim_end_matches(".git")
        .rsplit('/')
        .next()
        .unwrap_or("repository")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_repository_name_from_github_url() {
        assert_eq!(
            repo_name_from_url("https://github.com/user/project.git"),
            "project"
        );
    }
}
