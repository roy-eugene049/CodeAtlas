use codeatlas_domain::{Language, RepositoryIndex};
use codeatlas_graph::ArchitectureOutline;

/// Extractive repository blurb from the index — not an LLM guess.
pub fn summarize_repository(
    index: &RepositoryIndex,
    outline: &ArchitectureOutline,
    languages: &[(Language, u64)],
) -> String {
    let total = index.files.len().max(1) as f32;
    let mut parts = languages
        .iter()
        .map(|(language, count)| {
            format!(
                "{} {}%",
                language.as_str(),
                ((*count as f32 / total) * 100.0).round() as u32
            )
        })
        .collect::<Vec<_>>();
    parts.sort();
    let languages = if parts.is_empty() {
        "no detected languages".into()
    } else {
        parts.join(", ")
    };
    format!(
        "{} has {} files and {} symbols. {}. Languages: {}.",
        index.repository.name,
        index.files.len(),
        index.symbols.len(),
        outline.summary.trim_end_matches('.'),
        languages
    )
}

#[cfg(test)]
mod tests {
    use codeatlas_domain::{Language, Repository, RepositoryIndex};
    use codeatlas_graph::ArchitectureOutline;

    use super::*;

    #[test]
    fn summary_uses_index_counts() {
        let repo = Repository::new("pay", "u", "main", "sha");
        let index = RepositoryIndex {
            repository: repo,
            files: vec![],
            symbols: vec![],
            relationships: vec![],
            units: vec![],
            line_count: 0,
        };
        let text = summarize_repository(
            &index,
            &ArchitectureOutline {
                groups: vec![],
                summary: "Inferred from paths, not an LLM. Backend — 2 files.".into(),
            },
            &[(Language::TypeScript, 2)],
        );
        assert!(text.contains("pay"));
        assert!(text.contains("Backend"));
        assert!(text.contains("typescript"));
    }
}
