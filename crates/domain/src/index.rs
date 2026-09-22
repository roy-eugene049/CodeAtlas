use serde::{Deserialize, Serialize};

use crate::file::SourceFile;
use crate::relationship::{Relationship, RelationshipKind};
use crate::repository::Repository;
use crate::symbol::Symbol;
use crate::unit::SemanticUnit;

/// Complete in-memory representation of an indexed codebase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryIndex {
    pub repository: Repository,
    pub files: Vec<SourceFile>,
    pub symbols: Vec<Symbol>,
    pub relationships: Vec<Relationship>,
    pub units: Vec<SemanticUnit>,
    pub line_count: u64,
}

impl RepositoryIndex {
    pub fn stats(&self) -> IndexStats {
        let dependency_count = self
            .relationships
            .iter()
            .filter(|rel| rel.kind == RelationshipKind::Imports)
            .count() as u64;

        IndexStats {
            file_count: self.files.len() as u64,
            symbol_count: self.symbols.len() as u64,
            dependency_count,
            line_count: self.line_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStats {
    pub file_count: u64,
    pub symbol_count: u64,
    pub dependency_count: u64,
    pub line_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::FileId;
    use crate::language::Language;
    use crate::relationship::Relationship;
    use crate::repository::Repository;
    use crate::symbol::{Symbol, SymbolKind};

    #[test]
    fn stats_count_files_symbols_and_import_edges() {
        let repo = Repository::new("demo", "https://example.com/demo", "main", "sha");
        let file = SourceFile::new(repo.id, "src/a.ts", Language::TypeScript, 10, "h");
        let caller = Symbol::new(file.id, "AuthPage", SymbolKind::Component, 1, 10);
        let callee = Symbol::new(FileId::new(), "useAuth", SymbolKind::Function, 1, 4);
        let index = RepositoryIndex {
            repository: repo,
            files: vec![file],
            symbols: vec![caller.clone(), callee.clone()],
            relationships: vec![
                Relationship::new(caller.id, callee.id, RelationshipKind::Imports),
                Relationship::new(caller.id, callee.id, RelationshipKind::Calls),
            ],
            units: vec![],
            line_count: 42,
        };

        assert_eq!(
            index.stats(),
            IndexStats {
                file_count: 1,
                symbol_count: 2,
                dependency_count: 1,
                line_count: 42,
            }
        );
    }
}
