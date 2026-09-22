use std::sync::Mutex;

use codeatlas_domain::{
    unix_now, AnalysisRun, AnalysisRunId, GitEvolution, IndexProgress, IndexStage, Language,
    Relationship, RelationshipKind, Repository, RepositoryId, RepositoryIndex, SemanticUnit,
    SourceFile, Symbol, SymbolId, SymbolKind,
};
use postgres::{Client, NoTls, Row};

use crate::codec::{decode_f32s, encode_f32s, parse_pg};
use crate::error::StorageError;
use crate::schema::POSTGRES_SCHEMA;

/// PostgreSQL is the designed store. Opened when `CODEATLAS_DATABASE_URL` is a postgres URL.
pub struct PostgresStore {
    client: Mutex<Client>,
}

impl PostgresStore {
    pub fn connect(url: &str) -> Result<Self, StorageError> {
        let mut client = Client::connect(url, NoTls)?;
        client.batch_execute(POSTGRES_SCHEMA)?;
        Ok(Self {
            client: Mutex::new(client),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Client>, StorageError> {
        Ok(self.client.lock().unwrap_or_else(|error| error.into_inner()))
    }

    pub fn save(&self, index: &RepositoryIndex) -> Result<(), StorageError> {
        let mut client = self.lock()?;
        let mut tx = client.transaction()?;
        tx.execute(
            "DELETE FROM repositories WHERE id = $1",
            &[&index.repository.id.to_string()],
        )?;
        tx.execute(
            "INSERT INTO repositories (id, name, url, branch, commit_sha, created_at, line_count)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &[
                &index.repository.id.to_string(),
                &index.repository.name,
                &index.repository.url,
                &index.repository.default_branch,
                &index.repository.commit_sha,
                &unix_now(),
                &(index.line_count as i64),
            ],
        )?;
        for file in &index.files {
            tx.execute(
                "INSERT INTO files (id, repository_id, path, language, hash, size_bytes)
                 VALUES ($1, $2, $3, $4, $5, $6)",
                &[
                    &file.id.to_string(),
                    &file.repository_id.to_string(),
                    &file.path,
                    &file.language.as_str(),
                    &file.hash,
                    &(file.size_bytes as i64),
                ],
            )?;
        }
        for symbol in &index.symbols {
            tx.execute(
                "INSERT INTO symbols (id, file_id, name, kind, start_line, end_line)
                 VALUES ($1, $2, $3, $4, $5, $6)",
                &[
                    &symbol.id.to_string(),
                    &symbol.file_id.to_string(),
                    &symbol.name,
                    &symbol.kind.as_str(),
                    &(symbol.start_line as i32),
                    &(symbol.end_line as i32),
                ],
            )?;
        }
        for rel in &index.relationships {
            tx.execute(
                "INSERT INTO relationships (source_id, target_id, kind, repository_id)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &rel.source.to_string(),
                    &rel.target.to_string(),
                    &rel.kind.as_str(),
                    &index.repository.id.to_string(),
                ],
            )?;
        }
        for unit in &index.units {
            tx.execute(
                "INSERT INTO chunks (id, symbol_id, content, token_count) VALUES ($1, $2, $3, $4)",
                &[
                    &unit.symbol_id.to_string(),
                    &unit.symbol_id.to_string(),
                    &unit.text,
                    &(unit.token_count() as i32),
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn save_embeddings(
        &self,
        repository_id: RepositoryId,
        embeddings: &[(SymbolId, Vec<f32>)],
    ) -> Result<(), StorageError> {
        let mut client = self.lock()?;
        let mut tx = client.transaction()?;
        tx.execute(
            "DELETE FROM embeddings WHERE repository_id = $1",
            &[&repository_id.to_string()],
        )?;
        for (id, vector) in embeddings {
            let bytes = encode_f32s(vector);
            tx.execute(
                "INSERT INTO embeddings (chunk_id, repository_id, dim, vector) VALUES ($1, $2, $3, $4)",
                &[
                    &id.to_string(),
                    &repository_id.to_string(),
                    &(vector.len() as i32),
                    &bytes,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_embeddings(
        &self,
        repository_id: RepositoryId,
    ) -> Result<Vec<(SymbolId, Vec<f32>)>, StorageError> {
        let mut client = self.lock()?;
        let rows = client.query(
            "SELECT chunk_id, vector FROM embeddings WHERE repository_id = $1",
            &[&repository_id.to_string()],
        )?;
        rows.into_iter()
            .map(|row| {
                Ok((
                    parse_pg(row.get(0))?,
                    decode_f32s(row.get::<_, Vec<u8>>(1).as_slice()),
                ))
            })
            .collect()
    }

    pub fn list_repositories(&self) -> Result<Vec<(Repository, u64)>, StorageError> {
        let mut client = self.lock()?;
        let rows = client.query(
            "SELECT id, name, url, branch, commit_sha, line_count FROM repositories ORDER BY name",
            &[],
        )?;
        rows.into_iter()
            .map(|row| Ok((pg_repo(&row)?, row.get::<_, i64>(5) as u64)))
            .collect()
    }

    pub fn find_by_url(&self, url: &str) -> Result<Option<RepositoryId>, StorageError> {
        let mut client = self.lock()?;
        let rows = client.query(
            "SELECT id FROM repositories WHERE url = $1 ORDER BY created_at DESC LIMIT 1",
            &[&url],
        )?;
        rows.first()
            .map(|row| parse_pg(row.get(0)))
            .transpose()
    }

    pub fn find_symbol(
        &self,
        id: SymbolId,
    ) -> Result<Option<(RepositoryId, Symbol, String)>, StorageError> {
        let mut client = self.lock()?;
        let rows = client.query(
            "SELECT s.id, s.file_id, s.name, s.kind, s.start_line, s.end_line, f.repository_id, f.path
             FROM symbols s JOIN files f ON f.id = s.file_id WHERE s.id = $1",
            &[&id.to_string()],
        )?;
        rows.first().map(pg_symbol_hit).transpose()
    }

    pub fn load(&self, id: RepositoryId) -> Result<Option<RepositoryIndex>, StorageError> {
        let mut client = self.lock()?;
        let repos = client.query(
            "SELECT id, name, url, branch, commit_sha, line_count FROM repositories WHERE id = $1",
            &[&id.to_string()],
        )?;
        let Some(repo_row) = repos.first() else {
            return Ok(None);
        };
        let repository = pg_repo(repo_row)?;
        let line_count = repo_row.get::<_, i64>(5) as u64;

        let files = client
            .query(
                "SELECT id, repository_id, path, language, size_bytes, hash FROM files WHERE repository_id = $1",
                &[&id.to_string()],
            )?
            .into_iter()
            .map(|row| {
                Ok(SourceFile {
                    id: parse_pg(row.get(0))?,
                    repository_id: parse_pg(row.get(1))?,
                    path: row.get(2),
                    language: row
                        .get::<_, String>(3)
                        .parse::<Language>()
                        .map_err(|error| StorageError::Language(error.to_string()))?,
                    size_bytes: row.get::<_, i64>(4) as u64,
                    hash: row.get(5),
                })
            })
            .collect::<Result<Vec<_>, StorageError>>()?;

        let symbols = client
            .query(
                "SELECT s.id, s.file_id, s.name, s.kind, s.start_line, s.end_line
                 FROM symbols s JOIN files f ON f.id = s.file_id WHERE f.repository_id = $1",
                &[&id.to_string()],
            )?
            .into_iter()
            .map(pg_symbol)
            .collect::<Result<Vec<_>, StorageError>>()?;

        let relationships = client
            .query(
                "SELECT source_id, target_id, kind FROM relationships WHERE repository_id = $1",
                &[&id.to_string()],
            )?
            .into_iter()
            .map(|row| {
                Ok(Relationship {
                    source: parse_pg(row.get(0))?,
                    target: parse_pg(row.get(1))?,
                    kind: row
                        .get::<_, String>(2)
                        .parse::<RelationshipKind>()
                        .map_err(|error| StorageError::RelationshipKind(error.to_string()))?,
                })
            })
            .collect::<Result<Vec<_>, StorageError>>()?;

        let units = client
            .query(
                "SELECT c.symbol_id, f.repository_id, s.file_id, s.name, s.kind, f.path, f.language,
                        s.start_line, s.end_line, c.content
                 FROM chunks c
                 JOIN symbols s ON s.id = c.symbol_id
                 JOIN files f ON f.id = s.file_id
                 WHERE f.repository_id = $1",
                &[&id.to_string()],
            )?
            .into_iter()
            .map(|row| {
                Ok(SemanticUnit {
                    symbol_id: parse_pg(row.get(0))?,
                    repository_id: parse_pg(row.get(1))?,
                    file_id: parse_pg(row.get(2))?,
                    name: row.get(3),
                    kind: row
                        .get::<_, String>(4)
                        .parse::<SymbolKind>()
                        .map_err(|error| StorageError::SymbolKind(error.to_string()))?,
                    path: row.get(5),
                    language: row
                        .get::<_, String>(6)
                        .parse::<Language>()
                        .map_err(|error| StorageError::Language(error.to_string()))?,
                    start_line: row.get::<_, i32>(7) as u32,
                    end_line: row.get::<_, i32>(8) as u32,
                    text: row.get(9),
                })
            })
            .collect::<Result<Vec<_>, StorageError>>()?;

        Ok(Some(RepositoryIndex {
            repository,
            files,
            symbols,
            relationships,
            units,
            line_count,
        }))
    }

    pub fn save_evolution(
        &self,
        repository_id: RepositoryId,
        evolution: &GitEvolution,
    ) -> Result<(), StorageError> {
        let payload = serde_json::to_string(evolution)
            .map_err(|error| StorageError::Evolution(error.to_string()))?;
        let mut client = self.lock()?;
        client.execute(
            "INSERT INTO git_evolution (repository_id, payload) VALUES ($1, $2)
             ON CONFLICT (repository_id) DO UPDATE SET payload = EXCLUDED.payload",
            &[&repository_id.to_string(), &payload],
        )?;
        Ok(())
    }

    pub fn load_evolution(&self, repository_id: RepositoryId) -> Result<GitEvolution, StorageError> {
        let mut client = self.lock()?;
        let rows = client.query(
            "SELECT payload FROM git_evolution WHERE repository_id = $1",
            &[&repository_id.to_string()],
        )?;
        match rows.first() {
            Some(row) => serde_json::from_str(row.get::<_, String>(0).as_str())
                .map_err(|error| StorageError::Evolution(error.to_string())),
            None => Ok(GitEvolution::default()),
        }
    }

    pub fn save_run(&self, run: &AnalysisRun) -> Result<(), StorageError> {
        let mut client = self.lock()?;
        let repo_id = run.repository_id.map(|id| id.to_string());
        client.execute(
            "INSERT INTO analysis_runs (
                id, repository_id, source, status, stage, files_discovered, files_parsed, files_skipped,
                symbols_extracted, relationships_built, chunks_embedded, message, error, started_at, finished_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             ON CONFLICT (id) DO UPDATE SET
                repository_id = EXCLUDED.repository_id,
                status = EXCLUDED.status,
                stage = EXCLUDED.stage,
                files_discovered = EXCLUDED.files_discovered,
                files_parsed = EXCLUDED.files_parsed,
                files_skipped = EXCLUDED.files_skipped,
                symbols_extracted = EXCLUDED.symbols_extracted,
                relationships_built = EXCLUDED.relationships_built,
                chunks_embedded = EXCLUDED.chunks_embedded,
                message = EXCLUDED.message,
                error = EXCLUDED.error,
                finished_at = EXCLUDED.finished_at",
            &[
                &run.id.to_string(),
                &repo_id,
                &run.source,
                &run.status.as_str(),
                &run.progress.stage.as_str(),
                &(run.progress.files_discovered as i64),
                &(run.progress.files_parsed as i64),
                &(run.progress.files_skipped as i64),
                &(run.progress.symbols_extracted as i64),
                &(run.progress.relationships_built as i64),
                &(run.progress.chunks_embedded as i64),
                &run.progress.message,
                &run.error,
                &run.started_at,
                &run.finished_at,
            ],
        )?;
        Ok(())
    }

    pub fn load_run(&self, id: AnalysisRunId) -> Result<Option<AnalysisRun>, StorageError> {
        let mut client = self.lock()?;
        let rows = client.query(
            "SELECT id, repository_id, source, status, stage, files_discovered, files_parsed, files_skipped,
                    symbols_extracted, relationships_built, chunks_embedded, message, error, started_at, finished_at
             FROM analysis_runs WHERE id = $1",
            &[&id.to_string()],
        )?;
        rows.first().map(pg_run).transpose()
    }

    pub fn search_symbols(&self, id: RepositoryId, query: &str) -> Result<Vec<Symbol>, StorageError> {
        let mut client = self.lock()?;
        let pattern = format!("%{query}%");
        client
            .query(
                "SELECT s.id, s.file_id, s.name, s.kind, s.start_line, s.end_line
                 FROM symbols s JOIN files f ON f.id = s.file_id
                 WHERE f.repository_id = $1 AND LOWER(s.name) LIKE LOWER($2)
                 ORDER BY s.name LIMIT 200",
                &[&id.to_string(), &pattern],
            )?
            .into_iter()
            .map(pg_symbol)
            .collect()
    }
}

fn pg_repo(row: &Row) -> Result<Repository, StorageError> {
    Ok(Repository {
        id: parse_pg(row.get(0))?,
        name: row.get(1),
        url: row.get(2),
        default_branch: row.get(3),
        commit_sha: row.get(4),
    })
}

fn pg_symbol(row: Row) -> Result<Symbol, StorageError> {
    Ok(Symbol {
        id: parse_pg(row.get(0))?,
        file_id: parse_pg(row.get(1))?,
        name: row.get(2),
        kind: row
            .get::<_, String>(3)
            .parse::<SymbolKind>()
            .map_err(|error| StorageError::SymbolKind(error.to_string()))?,
        start_line: row.get::<_, i32>(4) as u32,
        end_line: row.get::<_, i32>(5) as u32,
    })
}

fn pg_symbol_hit(row: &Row) -> Result<(RepositoryId, Symbol, String), StorageError> {
    Ok((
        parse_pg(row.get(6))?,
        Symbol {
            id: parse_pg(row.get(0))?,
            file_id: parse_pg(row.get(1))?,
            name: row.get(2),
            kind: row
                .get::<_, String>(3)
                .parse::<SymbolKind>()
                .map_err(|error| StorageError::SymbolKind(error.to_string()))?,
            start_line: row.get::<_, i32>(4) as u32,
            end_line: row.get::<_, i32>(5) as u32,
        },
        row.get(7),
    ))
}

fn pg_run(row: &Row) -> Result<AnalysisRun, StorageError> {
    let repository_id = row
        .get::<_, Option<String>>(1)
        .map(parse_pg)
        .transpose()?;
    Ok(AnalysisRun {
        id: parse_pg(row.get(0))?,
        repository_id,
        source: row.get(2),
        status: row
            .get::<_, String>(3)
            .parse::<IndexStage>()
            .map_err(StorageError::Run)?,
        progress: IndexProgress {
            stage: row
                .get::<_, String>(4)
                .parse::<IndexStage>()
                .map_err(StorageError::Run)?,
            files_discovered: row.get::<_, i64>(5) as u64,
            files_parsed: row.get::<_, i64>(6) as u64,
            files_skipped: row.get::<_, i64>(7) as u64,
            symbols_extracted: row.get::<_, i64>(8) as u64,
            relationships_built: row.get::<_, i64>(9) as u64,
            chunks_embedded: row.get::<_, i64>(10) as u64,
            message: row.get(11),
            duration_ms: 0,
        },
        error: row.get(12),
        started_at: row.get(13),
        finished_at: row.get(14),
    })
}
