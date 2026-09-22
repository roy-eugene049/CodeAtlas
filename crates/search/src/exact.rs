use codeatlas_domain::RepositoryIndex;

use crate::embed::tokenize;
use crate::{SearchResult, SearchSource};

/// Exact / lexical search: symbol name, file name, path, and unit text.
pub fn exact_search(index: &RepositoryIndex, query: &str, limit: usize) -> Vec<SearchResult> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }
    let q = query.to_ascii_lowercase();
    let tokens = tokenize(&q);
    let mut hits = Vec::new();

    for symbol in &index.symbols {
        let Some(file) = index.files.iter().find(|file| file.id == symbol.file_id) else {
            continue;
        };
        let name = symbol.name.to_ascii_lowercase();
        let path = file.path.to_ascii_lowercase();
        let file_name = path.rsplit('/').next().unwrap_or(&path);
        let unit_text = index
            .units
            .iter()
            .find(|unit| unit.symbol_id == symbol.id)
            .map(|unit| unit.text.to_ascii_lowercase())
            .unwrap_or_default();

        let (score, source) = if name.contains(&q) {
            (1.0, SearchSource::SymbolName)
        } else if file_name.contains(&q) {
            (0.85, SearchSource::FileName)
        } else if path.contains(&q) {
            (0.7, SearchSource::Path)
        } else if !tokens.is_empty()
            && tokens
                .iter()
                .any(|token| name.contains(token) || unit_text.contains(token.as_str()))
        {
            (0.45, SearchSource::Text)
        } else {
            continue;
        };

        hits.push(SearchResult {
            symbol_id: symbol.id,
            name: symbol.name.clone(),
            path: file.path.clone(),
            kind: symbol.kind.as_str().to_string(),
            score,
            source,
        });
    }

    hits.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.name.cmp(&b.name)));
    hits.truncate(limit.max(1));
    hits
}
