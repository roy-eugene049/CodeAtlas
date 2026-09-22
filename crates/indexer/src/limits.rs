#[derive(Debug, Clone, Copy)]
pub struct IndexLimits {
    pub max_file_bytes: u64,
    pub max_file_count: usize,
    pub max_repo_bytes: u64,
}

impl IndexLimits {
    pub const DEFAULT_MAX_FILE_BYTES: u64 = 1024 * 1024;
    pub const DEFAULT_MAX_FILE_COUNT: usize = 100_000;
    pub const DEFAULT_MAX_REPO_BYTES: u64 = 512 * 1024 * 1024;

    pub fn from_env() -> Self {
        Self {
            max_file_bytes: env_u64("CODEATLAS_MAX_FILE_BYTES", Self::DEFAULT_MAX_FILE_BYTES),
            max_file_count: env_usize("CODEATLAS_MAX_FILE_COUNT", Self::DEFAULT_MAX_FILE_COUNT),
            max_repo_bytes: env_u64("CODEATLAS_MAX_REPO_BYTES", Self::DEFAULT_MAX_REPO_BYTES),
        }
    }
}

impl Default for IndexLimits {
    fn default() -> Self {
        Self::from_env()
    }
}

fn env_u64(name: &str, fallback: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

fn env_usize(name: &str, fallback: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}
