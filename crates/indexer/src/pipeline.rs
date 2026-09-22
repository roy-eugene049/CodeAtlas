use std::collections::{HashMap, HashSet};
use std::time::Instant;

use rayon::prelude::*;

use codeatlas_domain::{
    IndexProgress, IndexStage, Repository, RepositoryIndex, SourceFile, Symbol, SymbolId,
};
use codeatlas_parser::{ParsedFile, ParserRegistry};

use crate::error::IndexError;
use crate::resolve::{resolve, resolve_changed};
use crate::source::MaterializedSource;
use crate::units::build_units;
use crate::walk::{discover_files, DiscoveredFile};

const PARSE_THREADS: usize = 8;

pub struct Indexer {
    registry: ParserRegistry,
    parse_threads: usize,
}

#[derive(Debug, Clone)]
pub struct IndexOutcome {
    pub index: RepositoryIndex,
    pub progress: IndexProgress,
    pub changed_symbol_ids: Vec<SymbolId>,
}

impl Indexer {
    pub fn new(registry: ParserRegistry) -> Self {
        Self {
            registry,
            parse_threads: PARSE_THREADS,
        }
    }

    pub fn with_parse_threads(mut self, threads: usize) -> Self {
        self.parse_threads = threads.max(1);
        self
    }

    pub fn index(&self, source: &MaterializedSource) -> Result<RepositoryIndex, IndexError> {
        Ok(self.index_with_previous(source, None, |_| {})?.index)
    }

    pub fn index_with_previous(
        &self,
        source: &MaterializedSource,
        previous: Option<&RepositoryIndex>,
        on_progress: impl Fn(IndexProgress),
    ) -> Result<IndexOutcome, IndexError> {
        let started = Instant::now();
        tracing::info!(
            event = "index_started",
            repo = %source.name,
            "index started"
        );
        let mut progress = IndexProgress::queued();
        progress.stage = IndexStage::Scanning;
        progress.message = "Scanning repository…".into();
        on_progress(progress.clone());

        let discovered = discover_files(&source.root)?;
        if discovered.is_empty() {
            return Err(IndexError::EmptyRepository);
        }
        progress.files_discovered = discovered.len() as u64;
        progress.message = format!("{} files discovered", progress.files_discovered);
        on_progress(progress.clone());

        let repository = match previous {
            Some(index) => {
                let mut repo = index.repository.clone();
                repo.name = source.name.clone();
                repo.url = source.url.clone();
                repo.default_branch = source.default_branch.clone();
                repo.commit_sha = source.commit_sha.clone();
                repo
            }
            None => Repository::new(
                source.name.clone(),
                source.url.clone(),
                source.default_branch.clone(),
                source.commit_sha.clone(),
            ),
        };

        let previous_by_path = previous
            .map(|index| {
                index
                    .files
                    .iter()
                    .map(|file| (file.path.clone(), file.clone()))
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();

        let git_diff = previous.and_then(|index| {
            codeatlas_git::diff_index_range(
                &source.root,
                &index.repository.commit_sha,
                &source.commit_sha,
            )
            .ok()
        });
        if let Some(diff) = &git_diff {
            if !diff.is_empty() {
                progress.message = format!("Git delta: {}", diff.summary());
                on_progress(progress.clone());
            }
        }

        progress.stage = IndexStage::Parsing;
        progress.message = "Parsing changed files…".into();
        on_progress(progress.clone());

        let mut to_parse = Vec::new();
        let mut skipped_files = Vec::new();
        let mut file_sources: HashMap<String, String> = HashMap::new();
        let parse_only = git_diff.as_ref().map(|diff| diff.parse_paths());
        let drop_paths = git_diff.as_ref().map(|diff| diff.drop_paths());

        for file in &discovered {
            if drop_paths
                .as_ref()
                .is_some_and(|paths| paths.contains(&file.relative_path))
            {
                continue;
            }
            if let Some(only) = &parse_only {
                if !only.contains(&file.relative_path) {
                    if let Some(existing) = previous_by_path.get(&file.relative_path) {
                        skipped_files.push(existing.clone());
                        progress.files_skipped += 1;
                        continue;
                    }
                }
            }
            let source_text = std::fs::read_to_string(&file.absolute_path)?;
            let hash = blake3::hash(source_text.as_bytes()).to_hex().to_string();
            if let Some(existing) = previous_by_path.get(&file.relative_path) {
                if existing.hash == hash {
                    skipped_files.push(existing.clone());
                    progress.files_skipped += 1;
                    continue;
                }
            }
            file_sources.insert(file.relative_path.clone(), source_text);
            to_parse.push(file.clone());
        }

        let parsed = parse_files(&self.registry, &to_parse, &file_sources, self.parse_threads)?;
        progress.files_parsed = parsed.len() as u64;
        progress.message = format!(
            "{} files parsed, {} unchanged skipped",
            progress.files_parsed, progress.files_skipped
        );
        on_progress(progress.clone());

        let mut files = Vec::new();
        let mut parsed_files = Vec::new();
        let mut sources = Vec::new();
        let mut line_count = 0u64;
        let mut changed_symbol_ids = Vec::new();

        for existing in skipped_files {
            if let Some(prev) = previous {
                for symbol in prev.symbols.iter().filter(|symbol| symbol.file_id == existing.id) {
                    line_count = line_count.max(u64::from(symbol.end_line));
                }
            }
            files.push(existing);
        }

        for (discovered, parsed, loc, hash) in parsed {
            line_count += loc;
            let file = SourceFile::new(
                repository.id,
                discovered.relative_path.clone(),
                discovered.language,
                discovered.size_bytes,
                hash,
            );
            let source_text = file_sources
                .get(&discovered.relative_path)
                .cloned()
                .unwrap_or_default();
            sources.push((discovered.relative_path, source_text));
            files.push(file.clone());
            parsed_files.push((file, parsed));
        }

        progress.stage = IndexStage::Resolving;
        progress.message = "Building symbol graph…".into();
        on_progress(progress.clone());

        let (symbols, relationships) = if previous.is_some() && !parsed_files.is_empty() {
            merge_incremental(previous, &files, &parsed_files, &mut changed_symbol_ids)
        } else if previous.is_some() && parsed_files.is_empty() {
            let prev = previous.expect("previous");
            let live: HashSet<_> = files.iter().map(|file| file.id).collect();
            let symbols = prev
                .symbols
                .iter()
                .filter(|symbol| live.contains(&symbol.file_id))
                .cloned()
                .collect::<Vec<_>>();
            let live_ids: HashSet<_> = symbols.iter().map(|symbol| symbol.id).collect();
            let relationships = prev
                .relationships
                .iter()
                .filter(|rel| live_ids.contains(&rel.source) && live_ids.contains(&rel.target))
                .cloned()
                .collect();
            (symbols, relationships)
        } else {
            let resolved = resolve(&parsed_files);
            changed_symbol_ids = resolved.symbols.iter().map(|symbol| symbol.id).collect();
            (resolved.symbols, resolved.relationships)
        };

        let units = if let Some(prev) = previous {
            let changed_files: HashSet<_> = parsed_files.iter().map(|(file, _)| file.id).collect();
            let mut reused = prev
                .units
                .iter()
                .filter(|unit| {
                    files.iter().any(|file| file.id == unit.file_id) && !changed_files.contains(&unit.file_id)
                })
                .cloned()
                .collect::<Vec<_>>();
            reused.extend(build_units(&files, &symbols, &sources));
            reused
        } else {
            build_units(&files, &symbols, &sources)
        };

        progress.symbols_extracted = symbols.len() as u64;
        progress.relationships_built = relationships.len() as u64;
        progress.message = format!("{} symbols extracted", progress.symbols_extracted);
        tracing::info!(
            event = "symbol_extracted",
            count = progress.symbols_extracted,
            "symbols extracted"
        );
        tracing::info!(
            event = "graph_updated",
            relationships = progress.relationships_built,
            "graph updated"
        );
        on_progress(progress.clone());

        let index = RepositoryIndex {
            repository,
            files,
            symbols,
            relationships,
            units,
            line_count,
        };

        progress.duration_ms = started.elapsed().as_millis() as u64;
        Ok(IndexOutcome {
            index,
            progress,
            changed_symbol_ids,
        })
    }
}

impl Default for Indexer {
    fn default() -> Self {
        Self::new(ParserRegistry::new())
    }
}

fn merge_incremental(
    previous: Option<&RepositoryIndex>,
    files: &[SourceFile],
    changed: &[(SourceFile, ParsedFile)],
    changed_symbol_ids: &mut Vec<SymbolId>,
) -> (Vec<Symbol>, Vec<codeatlas_domain::Relationship>) {
    let live_files: HashSet<_> = files.iter().map(|file| file.id).collect();
    let changed_files: HashSet<_> = changed.iter().map(|(file, _)| file.id).collect();
    let mut symbols = previous
        .map(|index| {
            index
                .symbols
                .iter()
                .filter(|symbol| {
                    live_files.contains(&symbol.file_id) && !changed_files.contains(&symbol.file_id)
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let resolved_changed = resolve(changed);
    changed_symbol_ids.extend(resolved_changed.symbols.iter().map(|symbol| symbol.id));
    symbols.extend(resolved_changed.symbols);

    let live_symbols: HashSet<_> = symbols.iter().map(|symbol| symbol.id).collect();
    let mut relationships = previous
        .map(|index| {
            index
                .relationships
                .iter()
                .filter(|rel| {
                    let source_file = symbols
                        .iter()
                        .find(|symbol| symbol.id == rel.source)
                        .map(|symbol| symbol.file_id);
                    source_file.is_some_and(|file_id| !changed_files.contains(&file_id))
                        && live_symbols.contains(&rel.source)
                        && live_symbols.contains(&rel.target)
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    relationships.extend(resolve_changed(&symbols, files, changed));
    relationships.sort_by_key(|rel| {
        (
            rel.source.to_string(),
            rel.target.to_string(),
            rel.kind.as_str().to_string(),
        )
    });
    relationships.dedup();
    (symbols, relationships)
}

fn parse_files(
    registry: &ParserRegistry,
    files: &[DiscoveredFile],
    sources: &HashMap<String, String>,
    parse_threads: usize,
) -> Result<Vec<(DiscoveredFile, ParsedFile, u64, String)>, IndexError> {
    if files.is_empty() {
        return Ok(Vec::new());
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(parse_threads)
        .build()
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error.to_string()))?;

    pool.install(|| {
        files
            .par_iter()
            .map(|file| {
                let source = sources.get(&file.relative_path).cloned().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("missing source for {}", file.relative_path),
                    )
                })?;
                let parsed = registry
                    .parse_path(&file.relative_path, &source)
                    .map_err(|error| IndexError::Parse {
                        path: file.relative_path.clone(),
                        source: error,
                    })?;
                tracing::debug!(
                    event = "file_parsed",
                    path = %file.relative_path,
                    symbols = parsed.symbols.len(),
                    "file parsed"
                );
                let loc = source.lines().count() as u64;
                let hash = blake3::hash(source.as_bytes()).to_hex().to_string();
                Ok((file.clone(), parsed, loc, hash))
            })
            .collect()
    })
}
