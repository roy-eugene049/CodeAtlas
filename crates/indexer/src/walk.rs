use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use codeatlas_domain::{is_sensitive_path, Language};

use crate::error::IndexError;
use crate::limits::IndexLimits;

const SKIP_DIRS: &[&str] = &[
    ".git",
    ".data",
    ".next",
    ".turbo",
    ".venv",
    ".cache",
    "node_modules",
    "target",
    "dist",
    "build",
    "vendor",
    "__pycache__",
    "coverage",
    "out",
];

#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub relative_path: String,
    pub absolute_path: PathBuf,
    pub language: Language,
    pub size_bytes: u64,
}

pub fn discover_files(root: &Path) -> Result<Vec<DiscoveredFile>, IndexError> {
    discover_files_with_limits(root, IndexLimits::default())
}

pub fn discover_files_with_limits(
    root: &Path,
    limits: IndexLimits,
) -> Result<Vec<DiscoveredFile>, IndexError> {
    if !root.is_dir() {
        return Err(IndexError::InvalidRoot(root.display().to_string()));
    }

    let mut files = Vec::new();
    let mut repo_bytes = 0u64;
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            !(entry.file_type().is_some_and(|kind| kind.is_dir()) && SKIP_DIRS.contains(&name.as_ref()))
        })
        .build();

    for entry in walker {
        let entry = entry.map_err(|error| {
            std::io::Error::new(std::io::ErrorKind::Other, error.to_string())
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(language) = path.to_str().and_then(Language::from_path) else {
            continue;
        };
        let size_bytes = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        if size_bytes == 0 || size_bytes > limits.max_file_bytes {
            continue;
        }
        let relative_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        if is_sensitive_path(&relative_path) {
            tracing::debug!(event = "file_skipped_secret", path = %relative_path, "skipped secret path");
            continue;
        }
        repo_bytes = repo_bytes.saturating_add(size_bytes);
        if repo_bytes > limits.max_repo_bytes {
            return Err(IndexError::RepositoryTooLarge(limits.max_repo_bytes));
        }
        if files.len() >= limits.max_file_count {
            return Err(IndexError::TooManyFiles(limits.max_file_count));
        }
        tracing::debug!(
            event = "file_discovered",
            path = %relative_path,
            size_bytes,
            "file discovered"
        );
        files.push(DiscoveredFile {
            relative_path,
            absolute_path: path.to_path_buf(),
            language,
            size_bytes,
        });
    }

    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_env_and_ignored_directories() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("src")).expect("src");
        std::fs::create_dir_all(dir.path().join("node_modules")).expect("nm");
        std::fs::create_dir_all(dir.path().join(".cache")).expect("cache");
        std::fs::write(dir.path().join("src/app.ts"), "export const x = 1;\n").expect("app");
        std::fs::write(dir.path().join(".env"), "API_KEY=secret\n").expect("env");
        std::fs::write(dir.path().join("node_modules/lib.ts"), "export const y = 2;\n").expect("nm");
        std::fs::write(dir.path().join(".cache/out.ts"), "export const z = 3;\n").expect("cache");
        let files = discover_files(dir.path()).expect("discover");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "src/app.ts");
    }

    #[test]
    fn enforces_file_count_limit() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.ts"), "export const a = 1;\n").expect("a");
        std::fs::write(dir.path().join("b.ts"), "export const b = 1;\n").expect("b");
        let error = discover_files_with_limits(
            dir.path(),
            IndexLimits {
                max_file_bytes: 1024,
                max_file_count: 1,
                max_repo_bytes: 1024 * 1024,
            },
        )
        .expect_err("limit");
        assert!(matches!(error, IndexError::TooManyFiles(1)));
    }
}
