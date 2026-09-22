//! Exact and semantic search. Independent of any one vector database.

mod embed;
mod error;
mod exact;
mod semantic;

use codeatlas_domain::SymbolId;
use serde::{Deserialize, Serialize};

pub use embed::{cosine, expanded_query, query_tokens, tokenize, Embedder, HashedEmbedder};
pub use error::SearchError;
pub use exact::exact_search;
pub use semantic::{embed_index, embed_units, HashedSemanticSearch, SemanticSearch};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchSource {
    SymbolName,
    FileName,
    Path,
    Text,
    Semantic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub symbol_id: SymbolId,
    pub name: String,
    pub path: String,
    pub kind: String,
    pub score: f32,
    pub source: SearchSource,
}

#[cfg(test)]
mod tests {
    use codeatlas_domain::{
        Language, Repository, RepositoryIndex, SemanticUnit, SourceFile, Symbol, SymbolKind,
    };

    use super::*;

    fn index() -> RepositoryIndex {
        let repo = Repository::new("d", "u", "main", "sha");
        let file = SourceFile::new(repo.id, "src/auth/session.ts", Language::TypeScript, 20, "h");
        let symbol = Symbol::new(file.id, "loadSession", SymbolKind::Function, 1, 8);
        let unit = SemanticUnit {
            symbol_id: symbol.id,
            repository_id: repo.id,
            file_id: file.id,
            name: symbol.name.clone(),
            kind: symbol.kind,
            path: file.path.clone(),
            language: file.language,
            start_line: 1,
            end_line: 8,
            text: "export function loadSession() { return cookie; }".into(),
        };
        RepositoryIndex {
            repository: repo,
            files: vec![file],
            symbols: vec![symbol],
            relationships: vec![],
            units: vec![unit],
            line_count: 8,
        }
    }

    #[test]
    fn exact_search_hits_symbol_name_and_path() {
        let index = index();
        let by_name = exact_search(&index, "loadSession", 8);
        assert_eq!(by_name[0].source, SearchSource::SymbolName);
        let by_path = exact_search(&index, "session.ts", 8);
        assert_eq!(by_path[0].source, SearchSource::FileName);
        let by_text = exact_search(&index, "cookie", 8);
        assert_eq!(by_text[0].source, SearchSource::Text);
    }

    #[test]
    fn semantic_search_is_swappable_via_the_trait() {
        let index = index();
        let embeddings = embed_units(&HashedEmbedder::default(), &index.units).expect("embed");
        let search = HashedSemanticSearch::new(&index.units, &embeddings);
        let hits = SemanticSearch::search(&search, "session cookie", 4).expect("search");
        assert_eq!(hits[0].name, "loadSession");
        assert_eq!(hits[0].source, SearchSource::Semantic);
    }
}
