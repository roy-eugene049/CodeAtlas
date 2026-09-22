use std::collections::HashSet;

use codeatlas_domain::{is_sensitive_path, looks_like_secret, redact_secrets, RepositoryIndex};
use codeatlas_graph::CodeGraph;

use crate::answer::{citation_key, ContextConfidence};
use crate::intent::QueryIntent;
use crate::retrieve::ScoredUnit;

const MAX_CONTEXT_CHARS: usize = 12_000;

#[derive(Debug, Clone)]
pub struct ContextPack {
    pub question: String,
    pub intent: QueryIntent,
    pub architecture: Vec<String>,
    pub units: Vec<ScoredUnit>,
    pub relationships: Vec<String>,
}

impl ContextPack {
    pub fn build(
        question: &str,
        intent: QueryIntent,
        index: &RepositoryIndex,
        mut units: Vec<ScoredUnit>,
    ) -> Self {
        units.retain(|unit| {
            !is_sensitive_path(&unit.unit.path) && !looks_like_secret(&unit.unit.text)
        });

        let graph = CodeGraph::build(index);
        let ids: std::collections::HashSet<_> =
            units.iter().map(|unit| unit.unit.symbol_id).collect();
        let mut relationships = Vec::new();
        for rel in &index.relationships {
            if ids.contains(&rel.source) && ids.contains(&rel.target) {
                let source = index
                    .symbols
                    .iter()
                    .find(|symbol| symbol.id == rel.source)
                    .map(|symbol| symbol.name.as_str())
                    .unwrap_or("?");
                let target = index
                    .symbols
                    .iter()
                    .find(|symbol| symbol.id == rel.target)
                    .map(|symbol| symbol.name.as_str())
                    .unwrap_or("?");
                relationships.push(format!("{source} -{}-> {target}", rel.kind));
            }
        }
        relationships.sort();
        relationships.dedup();

        let architecture = graph
            .architecture()
            .into_iter()
            .filter(|layer| layer.file_count > 0)
            .map(|layer| format!("{} ({})", layer.kind.as_str(), layer.file_count))
            .collect();

        let mut pack = Self {
            question: question.to_string(),
            intent,
            architecture,
            units,
            relationships,
        };
        pack.truncate();
        pack
    }

    pub fn confidence(&self) -> ContextConfidence {
        ContextConfidence::from_counts(self.units.len(), self.file_count())
    }

    pub fn file_count(&self) -> usize {
        self.units
            .iter()
            .map(|unit| unit.unit.path.as_str())
            .collect::<HashSet<_>>()
            .len()
    }

    pub fn retrieval_summary(&self) -> String {
        crate::answer::retrieval_summary(self.units.len(), self.file_count())
    }

    fn truncate(&mut self) {
        let mut used = 0usize;
        self.units.retain(|unit| {
            used += unit.unit.text.len();
            used <= MAX_CONTEXT_CHARS
        });
    }

    pub fn render(&self) -> String {
        let mut out = String::from("CODEBASE CONTEXT\n\n");
        out.push_str("Architecture:\n");
        for layer in &self.architecture {
            out.push_str(&format!("- {layer}\n"));
        }
        out.push_str("\nRelevant symbols:\n");
        for scored in &self.units {
            let unit = &scored.unit;
            out.push_str(&format!(
                "- [citation:{}] {} ({}) {}:{}\n{}\n\n",
                citation_key(unit.symbol_id),
                unit.name,
                unit.kind,
                unit.path,
                unit.start_line,
                redact_secrets(&unit.text)
            ));
        }
        out.push_str("Relationships:\n");
        for rel in &self.relationships {
            out.push_str(&format!("- {rel}\n"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use codeatlas_domain::{Language, SemanticUnit, SymbolId, SymbolKind};

    use super::*;
    use crate::retrieve::{RetrievalSource, ScoredUnit};

    fn unit(name: &str, path: &str, text: &str) -> ScoredUnit {
        ScoredUnit {
            score: 1.0,
            source: RetrievalSource::Semantic,
            unit: SemanticUnit {
                symbol_id: SymbolId::new(),
                repository_id: codeatlas_domain::RepositoryId::new(),
                file_id: codeatlas_domain::FileId::new(),
                name: name.into(),
                kind: SymbolKind::Function,
                path: path.into(),
                language: Language::TypeScript,
                start_line: 1,
                end_line: 4,
                text: text.into(),
            },
        }
    }

    #[test]
    fn drops_env_files_from_the_pack() {
        let index = RepositoryIndex {
            repository: codeatlas_domain::Repository::new("d", "u", "main", "sha"),
            files: vec![],
            symbols: vec![],
            relationships: vec![],
            units: vec![],
            line_count: 0,
        };
        let pack = ContextPack::build(
            "secrets?",
            QueryIntent::Search,
            &index,
            vec![
                unit("handleFailure", "src/pay.ts", "retry()"),
                unit("KEY", ".env", "API_KEY=secret"),
            ],
        );
        assert_eq!(pack.units.len(), 1);
        assert_eq!(pack.units[0].unit.name, "handleFailure");
        assert!(!pack.render().contains("API_KEY"));
    }
}
