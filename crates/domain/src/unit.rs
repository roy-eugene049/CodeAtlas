use serde::{Deserialize, Serialize};

use crate::ids::{FileId, RepositoryId, SymbolId};
use crate::language::Language;
use crate::symbol::SymbolKind;

/// One embeddable slice of a codebase: a single symbol, never a whole file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticUnit {
    pub symbol_id: SymbolId,
    pub repository_id: RepositoryId,
    pub file_id: FileId,
    pub name: String,
    pub kind: SymbolKind,
    pub path: String,
    pub language: Language,
    pub start_line: u32,
    pub end_line: u32,
    pub text: String,
}

impl SemanticUnit {
    pub fn token_count(&self) -> u32 {
        self.text.split_whitespace().count() as u32
    }

    pub fn embed_text(&self) -> String {
        format!(
            "{} {} {} {} {}",
            self.name,
            self.kind,
            self.path,
            self.language,
            self.text
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::FileId;
    use crate::symbol::SymbolKind;

    #[test]
    fn embed_text_includes_symbol_not_only_path() {
        let unit = SemanticUnit {
            symbol_id: SymbolId::new(),
            repository_id: RepositoryId::new(),
            file_id: FileId::new(),
            name: "handleFailure".into(),
            kind: SymbolKind::Method,
            path: "src/services/payment.ts".into(),
            language: Language::TypeScript,
            start_line: 143,
            end_line: 181,
            text: "handleFailure(error) { this.retry(); }".into(),
        };
        let text = unit.embed_text();
        assert!(text.contains("handleFailure"));
        assert!(text.contains("retry"));
        assert!(!text.contains("entire file"));
    }
}
