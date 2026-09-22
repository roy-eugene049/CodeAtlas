use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    TypeScript,
    JavaScript,
    Rust,
    Python,
}

impl Language {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Rust => "rust",
            Self::Python => "python",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "ts" | "tsx" | "mts" | "cts" => Some(Self::TypeScript),
            "js" | "jsx" | "mjs" | "cjs" => Some(Self::JavaScript),
            "rs" => Some(Self::Rust),
            "py" | "pyi" => Some(Self::Python),
            _ => None,
        }
    }

    pub fn from_path(path: &str) -> Option<Self> {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())?;
        Self::from_extension(ext)
    }

    pub fn uses_jsx(path: &str) -> bool {
        matches!(
            std::path::Path::new(path)
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_ascii_lowercase()),
            Some(ext) if matches!(ext.as_str(), "tsx" | "jsx")
        )
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Language {
    type Err = UnknownLanguage;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "typescript" => Ok(Self::TypeScript),
            "javascript" => Ok(Self::JavaScript),
            "rust" => Ok(Self::Rust),
            "python" => Ok(Self::Python),
            other => Err(UnknownLanguage(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownLanguage(pub String);

impl std::fmt::Display for UnknownLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown language: {}", self.0)
    }
}

impl std::error::Error for UnknownLanguage {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_phase_one_languages_from_path() {
        assert_eq!(Language::from_path("src/app.tsx"), Some(Language::TypeScript));
        assert_eq!(Language::from_path("lib/hooks.js"), Some(Language::JavaScript));
        assert_eq!(Language::from_path("crates/api/src/lib.rs"), Some(Language::Rust));
        assert_eq!(Language::from_path("services/auth.py"), Some(Language::Python));
        assert_eq!(Language::from_path("README.md"), None);
    }

    #[test]
    fn jsx_is_detected_from_extension() {
        assert!(Language::uses_jsx("ui/Button.tsx"));
        assert!(Language::uses_jsx("ui/Button.jsx"));
        assert!(!Language::uses_jsx("ui/Button.ts"));
    }
}
