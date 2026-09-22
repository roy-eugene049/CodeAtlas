use std::path::Path;
use std::process::Command;

use crate::error::GitError;

pub fn stdout(cwd: &Path, args: &[&str]) -> Result<String, GitError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()?;
    if !output.status.success() {
        return Err(GitError::Command(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn git_root(path: &Path) -> Result<std::path::PathBuf, GitError> {
    let root = stdout(path, &["rev-parse", "--show-toplevel"])?;
    if root.is_empty() {
        return Err(GitError::NotARepository(path.display().to_string()));
    }
    Ok(std::path::PathBuf::from(root))
}
