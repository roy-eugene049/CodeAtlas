use codeatlas_domain::{RepositoryIndex, SemanticUnit, SymbolId};

use crate::embed::{cosine, expanded_query, Embedder, HashedEmbedder};
use crate::error::SearchError;
use crate::{SearchResult, SearchSource};

/// Vector search over semantic units. Implementations can be a local hasher,
/// a remote store, or a future vector database. Callers stay the same.
pub trait SemanticSearch: Send + Sync {
    fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError>;
}

pub struct HashedSemanticSearch<'a> {
    embedder: HashedEmbedder,
    units: &'a [SemanticUnit],
    embeddings: &'a [(SymbolId, Vec<f32>)],
}

impl<'a> HashedSemanticSearch<'a> {
    pub fn new(units: &'a [SemanticUnit], embeddings: &'a [(SymbolId, Vec<f32>)]) -> Self {
        Self {
            embedder: HashedEmbedder::default(),
            units,
            embeddings,
        }
    }
}

impl SemanticSearch for HashedSemanticSearch<'_> {
    fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let query_vec = self
            .embedder
            .embed(&[expanded_query(query)])?
            .into_iter()
            .next()
            .unwrap_or_default();
        let mut scored = self
            .embeddings
            .iter()
            .filter_map(|(id, vector)| {
                let unit = self.units.iter().find(|unit| unit.symbol_id == *id)?;
                Some(SearchResult {
                    symbol_id: *id,
                    name: unit.name.clone(),
                    path: unit.path.clone(),
                    kind: unit.kind.as_str().to_string(),
                    score: cosine(&query_vec, vector),
                    source: SearchSource::Semantic,
                })
            })
            .collect::<Vec<_>>();
        scored.sort_by(|left, right| right.score.total_cmp(&left.score));
        scored.truncate(limit.max(1));
        Ok(scored)
    }
}

type EmbeddingRow = (SymbolId, Vec<f32>);

pub fn embed_units(
    embedder: &impl Embedder,
    units: &[SemanticUnit],
) -> Result<Vec<EmbeddingRow>, SearchError> {
    if units.is_empty() {
        return Ok(Vec::new());
    }
    let texts = units.iter().map(SemanticUnit::embed_text).collect::<Vec<_>>();
    let vectors = embedder.embed(&texts)?;
    Ok(units
        .iter()
        .zip(vectors)
        .map(|(unit, vector)| (unit.symbol_id, vector))
        .collect())
}

pub fn embed_index(
    embedder: &impl Embedder,
    index: &RepositoryIndex,
) -> Result<Vec<EmbeddingRow>, SearchError> {
    embed_units(embedder, &index.units)
}
