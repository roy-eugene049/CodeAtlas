use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use codeatlas_ai::{
    detect_intent, embed_units, summarize_repository, ContextEngine, FallbackModel, HashedEmbedder,
    QueryIntent,
};
use codeatlas_domain::{GitEvolution, RepositoryId, RepositoryIndex, SymbolId, SymbolKind};
use codeatlas_graph::{CodeGraph, SymbolRef};
use codeatlas_domain::{IndexProgress, IndexStage};
use codeatlas_indexer::{materialize, Indexer};
use codeatlas_search::{exact_search, HashedSemanticSearch, SemanticSearch};
use codeatlas_storage::Store;

use crate::dto::{
    AskResponse, ExplainResponse, FileContents, FileResponse, GraphResponse, ImpactResponse,
    LanguageCount, OverviewResponse, RepositorySummary, SearchHit, SymbolResponse,
};
use crate::llm::OpenAiClient;
use crate::error::ApiError;

pub struct RepositoryService {
    store: Arc<Store>,
    cache_dir: PathBuf,
    engine: ContextEngine<HashedEmbedder, FallbackModel>,
}

impl RepositoryService {
    pub fn new(store: Arc<Store>, cache_dir: PathBuf) -> Self {
        let model = OpenAiClient::from_env()
            .map(|client| FallbackModel::with_client(Arc::new(client)))
            .unwrap_or_else(FallbackModel::extractive);
        Self {
            store,
            cache_dir,
            engine: ContextEngine::new(HashedEmbedder::default(), model),
        }
    }

    pub fn run_index(
        &self,
        source: &str,
        on_progress: impl Fn(IndexProgress),
    ) -> Result<(OverviewResponse, RepositoryId), ApiError> {
        on_progress(IndexProgress {
            stage: IndexStage::Scanning,
            message: "Indexing repository…".into(),
            ..IndexProgress::queued()
        });
        let materialized = materialize(source, &self.cache_dir)?;
        let previous = self
            .store
            .find_by_url(&materialized.url)?
            .map(|id| self.store.load(id))
            .transpose()?
            .flatten();
        let outcome = Indexer::default().index_with_previous(
            &materialized,
            previous.as_ref(),
            |progress| {
                on_progress(progress);
            },
        )?;
        let mut index = outcome.index;
        if let Some(prev) = &previous {
            index.repository.id = prev.repository.id;
            for file in &mut index.files {
                file.repository_id = prev.repository.id;
            }
        }

        let mut progress = outcome.progress;
        progress.stage = IndexStage::Embedding;
        progress.message = "Generating embeddings…".into();
        on_progress(progress.clone());

        let changed: std::collections::HashSet<_> =
            outcome.changed_symbol_ids.into_iter().collect();
        let mut embeddings = self
            .store
            .load_embeddings(index.repository.id)?
            .into_iter()
            .filter(|(id, _)| {
                index.units.iter().any(|unit| unit.symbol_id == *id) && !changed.contains(id)
            })
            .collect::<Vec<_>>();
        let fresh = index
            .units
            .iter()
            .filter(|unit| {
                changed.contains(&unit.symbol_id)
                    || !embeddings.iter().any(|(id, _)| *id == unit.symbol_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        if !fresh.is_empty() {
            embeddings.extend(embed_units(&HashedEmbedder::default(), &fresh)?);
        }
        progress.chunks_embedded = embeddings.len() as u64;
        tracing::info!(
            event = "embedding_created",
            count = progress.chunks_embedded,
            "embeddings ready"
        );
        progress.stage = IndexStage::Persisting;
        progress.message = "Writing index…".into();
        on_progress(progress.clone());

        let evolution = codeatlas_git::collect_evolution(&materialized.root)?;
        self.store.save(&index)?;
        self.store
            .save_embeddings(index.repository.id, &embeddings)?;
        self.store
            .save_evolution(index.repository.id, &evolution)?;
        let duration_s = progress.duration_ms as f64 / 1000.0;
        progress.stage = IndexStage::Succeeded;
        progress.message = format!(
            "Index duration: {duration_s:.2}s · Files: {} · Parsed: {} · Skipped: {} · Symbols: {} · Relationships: {} · Embeddings: {}",
            progress.files_discovered,
            progress.files_parsed,
            progress.files_skipped,
            progress.symbols_extracted,
            progress.relationships_built,
            progress.chunks_embedded
        );
        tracing::info!(
            event = "index_completed",
            duration_secs = duration_s,
            files = progress.files_discovered,
            parsed = progress.files_parsed,
            skipped = progress.files_skipped,
            symbols = progress.symbols_extracted,
            relationships = progress.relationships_built,
            embeddings = progress.chunks_embedded,
            "index completed"
        );
        on_progress(progress);
        Ok((overview_from_index(&index), index.repository.id))
    }

    pub fn list(&self) -> Result<Vec<RepositorySummary>, ApiError> {
        let mut seen = std::collections::HashSet::new();
        Ok(self
            .store
            .list_repositories()?
            .into_iter()
            .filter(|(repo, _)| seen.insert(repo.url.clone()))
            .map(|(repo, line_count)| RepositorySummary::from_repo(repo, line_count))
            .collect())
    }

    pub fn overview(&self, id: RepositoryId) -> Result<OverviewResponse, ApiError> {
        Ok(overview_from_index(&self.load(id)?))
    }

    pub fn file_contents(&self, id: RepositoryId, path: &str) -> Result<FileContents, ApiError> {
        let path = normalize_repo_path(path)?;
        if codeatlas_domain::is_sensitive_path(&path) {
            return Err(ApiError::not_found("file not found"));
        }
        let index = self.load(id)?;
        let file = index
            .files
            .iter()
            .find(|file| file.path == path)
            .ok_or_else(|| ApiError::not_found("file not found"))?;
        let root = source_root(&index.repository.url, &self.cache_dir, &index.repository.name);
        let content = std::fs::read_to_string(root.join(&path))
            .map_err(|_| ApiError::not_found("file contents are not on disk"))?;
        Ok(FileContents {
            path,
            language: file.language,
            content,
        })
    }

    pub fn files(&self, id: RepositoryId) -> Result<Vec<FileResponse>, ApiError> {
        let mut files = self
            .load(id)?
            .files
            .into_iter()
            .map(FileResponse::from)
            .collect::<Vec<_>>();
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }

    pub fn symbols(
        &self,
        id: RepositoryId,
        query: Option<&str>,
        kind: Option<SymbolKind>,
    ) -> Result<Vec<SymbolResponse>, ApiError> {
        let index = self.load(id)?;
        let paths = index
            .files
            .iter()
            .map(|file| (file.id, file.path.clone()))
            .collect::<std::collections::HashMap<_, _>>();
        let mut symbols = if let Some(query) = query.filter(|q| !q.is_empty()) {
            let hits = exact_search(&index, query, 200);
            let wanted = hits
                .into_iter()
                .map(|hit| hit.symbol_id)
                .collect::<std::collections::HashSet<_>>();
            index
                .symbols
                .iter()
                .filter(|symbol| wanted.contains(&symbol.id))
                .cloned()
                .collect()
        } else {
            index.symbols
        };
        if let Some(kind) = kind {
            symbols.retain(|symbol| symbol.kind == kind);
        }
        symbols.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(symbols
            .into_iter()
            .map(|symbol| {
                let path = paths.get(&symbol.file_id).cloned().unwrap_or_default();
                SymbolResponse::from_symbol(symbol, path)
            })
            .collect())
    }

    pub fn graph(
        &self,
        id: RepositoryId,
        focus: Option<SymbolId>,
        depth: u8,
    ) -> Result<GraphResponse, ApiError> {
        let index = self.load(id)?;
        let graph = CodeGraph::build(&index);
        let view = match focus {
            Some(symbol_id) => {
                if graph.symbol(symbol_id).is_none() {
                    return Err(ApiError::not_found("symbol not found"));
                }
                graph.neighborhood(symbol_id, depth.max(1))
            }
            None => graph.compact_view(48),
        };
        Ok(GraphResponse { view, focus })
    }

    pub fn impact(&self, id: RepositoryId, symbol_id: SymbolId) -> Result<ImpactResponse, ApiError> {
        let index = self.load(id)?;
        let report = CodeGraph::build(&index)
            .impact(symbol_id)
            .ok_or_else(|| ApiError::not_found("symbol not found"))?;
        let narrative = self.engine.explain_impact(&report, &index);
        Ok(ImpactResponse { report, narrative })
    }

    pub fn explain(&self, id: RepositoryId, symbol_id: SymbolId) -> Result<ExplainResponse, ApiError> {
        let index = self.load(id)?;
        let symbol = index
            .symbols
            .iter()
            .find(|symbol| symbol.id == symbol_id)
            .cloned()
            .ok_or_else(|| ApiError::not_found("symbol not found"))?;
        let path = index
            .files
            .iter()
            .find(|file| file.id == symbol.file_id)
            .map(|file| file.path.clone())
            .unwrap_or_default();
        let explanation = self.engine.explain_symbol(symbol_id, &index)?;
        Ok(ExplainResponse::from_parts(
            SymbolResponse::from_symbol(symbol, path),
            explanation,
        ))
    }

    pub fn ask(&self, id: RepositoryId, question: &str) -> Result<AskResponse, ApiError> {
        let question = question.trim();
        if question.is_empty() || question.chars().count() > 500 {
            return Err(ApiError::bad_request("question must be 1–500 characters"));
        }
        let index = self.load(id)?;
        let answer = if detect_intent(question) == QueryIntent::Evolution {
            let evolution = self.store.load_evolution(id)?;
            self.engine.ask_evolution(question, &evolution, &index)?
        } else {
            let mut embeddings = self.store.load_embeddings(id)?;
            if embeddings.is_empty() && !index.units.is_empty() {
                embeddings = embed_units(&HashedEmbedder::default(), &index.units)?;
            }
            self.engine.ask(question, &index, &embeddings)?
        };
        Ok(AskResponse::from_answer(question.to_string(), answer))
    }

    pub fn search(&self, id: RepositoryId, query: &str) -> Result<Vec<SearchHit>, ApiError> {
        let index = self.load(id)?;
        let mut hits = exact_search(&index, query, 32);
        let embeddings = self.store.load_embeddings(id)?;
        if !embeddings.is_empty() {
            let semantic = HashedSemanticSearch::new(&index.units, &embeddings);
            hits.extend(semantic.search(query, 16)?);
        }
        hits.sort_by(|a, b| b.score.total_cmp(&a.score));
        let mut seen = std::collections::HashSet::new();
        hits.retain(|hit| seen.insert(hit.symbol_id));
        hits.truncate(40);
        Ok(hits.into_iter().map(SearchHit::from).collect())
    }

    pub fn dependencies(&self, id: RepositoryId, symbol_id: SymbolId) -> Result<Vec<SymbolRef>, ApiError> {
        let graph = CodeGraph::build(&self.load(id)?);
        if graph.symbol(symbol_id).is_none() {
            return Err(ApiError::not_found("symbol not found"));
        }
        Ok(graph.dependencies(symbol_id))
    }

    pub fn dependents(&self, id: RepositoryId, symbol_id: SymbolId) -> Result<Vec<SymbolRef>, ApiError> {
        let graph = CodeGraph::build(&self.load(id)?);
        if graph.symbol(symbol_id).is_none() {
            return Err(ApiError::not_found("symbol not found"));
        }
        Ok(graph.dependents(symbol_id))
    }

    pub fn related(&self, id: RepositoryId, symbol_id: SymbolId) -> Result<Vec<SymbolRef>, ApiError> {
        let graph = CodeGraph::build(&self.load(id)?);
        if graph.symbol(symbol_id).is_none() {
            return Err(ApiError::not_found("symbol not found"));
        }
        Ok(graph.related(symbol_id))
    }

    pub fn symbol(&self, symbol_id: SymbolId) -> Result<SymbolResponse, ApiError> {
        let (repo_id, symbol, path) = self
            .store
            .find_symbol(symbol_id)?
            .ok_or_else(|| ApiError::not_found("symbol not found"))?;
        let _ = repo_id;
        Ok(SymbolResponse::from_symbol(symbol, path))
    }

    pub fn impact_by_symbol(&self, symbol_id: SymbolId) -> Result<ImpactResponse, ApiError> {
        let (repo_id, _, _) = self
            .store
            .find_symbol(symbol_id)?
            .ok_or_else(|| ApiError::not_found("symbol not found"))?;
        self.impact(repo_id, symbol_id)
    }

    pub fn explain_by_symbol(&self, symbol_id: SymbolId) -> Result<ExplainResponse, ApiError> {
        let (repo_id, _, _) = self
            .store
            .find_symbol(symbol_id)?
            .ok_or_else(|| ApiError::not_found("symbol not found"))?;
        self.explain(repo_id, symbol_id)
    }

    pub fn evolution(&self, id: RepositoryId) -> Result<GitEvolution, ApiError> {
        let _ = self.load(id)?;
        Ok(self.store.load_evolution(id)?)
    }

    fn load(&self, id: RepositoryId) -> Result<RepositoryIndex, ApiError> {
        self.store
            .load(id)?
            .ok_or_else(|| ApiError::not_found("repository not found"))
    }
}

fn overview_from_index(index: &RepositoryIndex) -> OverviewResponse {
    let graph = CodeGraph::build(index);
    let mut languages = BTreeMap::new();
    for file in &index.files {
        *languages.entry(file.language).or_insert(0u64) += 1;
    }
    let total = index.files.len().max(1) as f32;
    let language_pairs = languages
        .iter()
        .map(|(language, count)| (*language, *count))
        .collect::<Vec<_>>();
    let outline = graph.architecture_outline();
    let ai_summary = summarize_repository(index, &outline, &language_pairs);

    OverviewResponse {
        repository: RepositorySummary::from_repo(index.repository.clone(), index.line_count),
        stats: index.stats(),
        architecture: graph.architecture(),
        architecture_tree: outline,
        health: graph.health(index),
        languages: languages
            .into_iter()
            .map(|(language, file_count)| LanguageCount {
                language,
                file_count,
                percent: file_count as f32 / total,
            })
            .collect(),
        ai_summary,
    }
}

fn normalize_repo_path(path: &str) -> Result<String, ApiError> {
    let path = path.replace('\\', "/");
    if path.is_empty() || path.starts_with('/') || path.split('/').any(|part| part == "..") {
        return Err(ApiError::bad_request("invalid file path"));
    }
    Ok(path)
}

fn source_root(url: &str, cache_dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let local = std::path::Path::new(url);
    if local.is_dir() {
        local.to_path_buf()
    } else {
        cache_dir.join(name)
    }
}
