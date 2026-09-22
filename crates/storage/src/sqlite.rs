use std::path::Path;
use std::sync::Mutex;

use codeatlas_domain::{
    unix_now, AnalysisRun, AnalysisRunId, GitEvolution, IndexProgress, IndexStage, Language,
    Relationship, RelationshipKind, Repository, RepositoryId, RepositoryIndex, SemanticUnit,
    SourceFile, Symbol, SymbolId, SymbolKind,
};
use rusqlite::{params, Connection, OptionalExtension};

use crate::codec::{decode_f32s, encode_f32s, parse_enum, parse_id};
use crate::error::StorageError;
use crate::schema::SQLITE_SCHEMA;

pub struct SqliteStore {
    conn: Mutex<Connection>,
}

impl SqliteStore {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| StorageError::Database(error.to_string()))?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(SQLITE_SCHEMA)?;
        migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, StorageError> {
        Ok(self.conn.lock().unwrap_or_else(|error| error.into_inner()))
    }

    pub fn save(&self, index: &RepositoryIndex) -> Result<(), StorageError> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM repositories WHERE id = ?1", [index.repository.id.to_string()])?;
        tx.execute(
            "INSERT INTO repositories (id, name, url, branch, commit_sha, created_at, line_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                index.repository.id.to_string(),
                index.repository.name,
                index.repository.url,
                index.repository.default_branch,
                index.repository.commit_sha,
                unix_now(),
                index.line_count as i64
            ],
        )?;
        for file in &index.files {
            tx.execute(
                "INSERT INTO files (id, repository_id, path, language, hash, size_bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    file.id.to_string(),
                    file.repository_id.to_string(),
                    file.path,
                    file.language.as_str(),
                    file.hash,
                    file.size_bytes as i64
                ],
            )?;
        }
        for symbol in &index.symbols {
            tx.execute(
                "INSERT INTO symbols (id, file_id, repository_id, name, kind, start_line, end_line)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    symbol.id.to_string(),
                    symbol.file_id.to_string(),
                    index.repository.id.to_string(),
                    symbol.name,
                    symbol.kind.as_str(),
                    symbol.start_line,
                    symbol.end_line
                ],
            )?;
        }
        for rel in &index.relationships {
            tx.execute(
                "INSERT INTO relationships (source_id, target_id, kind, repository_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    rel.source.to_string(),
                    rel.target.to_string(),
                    rel.kind.as_str(),
                    index.repository.id.to_string()
                ],
            )?;
        }
        for unit in &index.units {
            tx.execute(
                "INSERT INTO chunks (id, symbol_id, content, token_count) VALUES (?1, ?2, ?3, ?4)",
                params![
                    unit.symbol_id.to_string(),
                    unit.symbol_id.to_string(),
                    unit.text,
                    unit.token_count() as i64
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
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM embeddings WHERE repository_id = ?1", [repository_id.to_string()])?;
        for (id, vector) in embeddings {
            tx.execute(
                "INSERT INTO embeddings (chunk_id, repository_id, dim, vector) VALUES (?1, ?2, ?3, ?4)",
                params![
                    id.to_string(),
                    repository_id.to_string(),
                    vector.len() as i64,
                    encode_f32s(vector)
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
        let conn = self.lock()?;
        let mut stmt = conn.prepare("SELECT chunk_id, vector FROM embeddings WHERE repository_id = ?1")?;
        let rows = stmt.query_map([repository_id.to_string()], |row| {
            Ok((
                parse_id::<SymbolId>(row.get::<_, String>(0)?)?,
                decode_f32s(&row.get::<_, Vec<u8>>(1)?),
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(StorageError::from)
    }

    pub fn list_repositories(&self) -> Result<Vec<(Repository, u64)>, StorageError> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, url, branch, commit_sha, line_count FROM repositories ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| Ok((row_repo(row)?, row.get::<_, i64>(5)? as u64)))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(StorageError::from)
    }

    pub fn find_by_url(&self, url: &str) -> Result<Option<RepositoryId>, StorageError> {
        let conn = self.lock()?;
        conn.query_row(
            "SELECT id FROM repositories WHERE url = ?1 ORDER BY created_at DESC LIMIT 1",
            [url],
            |row| parse_id(row.get::<_, String>(0)?),
        )
        .optional()
        .map_err(StorageError::from)
    }

    pub fn find_symbol(
        &self,
        id: SymbolId,
    ) -> Result<Option<(RepositoryId, Symbol, String)>, StorageError> {
        let conn = self.lock()?;
        conn.query_row(
            "SELECT s.id, s.file_id, s.name, s.kind, s.start_line, s.end_line, f.repository_id, f.path
             FROM symbols s JOIN files f ON f.id = s.file_id WHERE s.id = ?1",
            [id.to_string()],
            |row| {
                Ok((
                    parse_id(row.get::<_, String>(6)?)?,
                    Symbol {
                        id: parse_id(row.get::<_, String>(0)?)?,
                        file_id: parse_id(row.get::<_, String>(1)?)?,
                        name: row.get(2)?,
                        kind: parse_enum::<SymbolKind, _>(row.get::<_, String>(3)?, StorageError::SymbolKind)?,
                        start_line: row.get::<_, i64>(4)? as u32,
                        end_line: row.get::<_, i64>(5)? as u32,
                    },
                    row.get(7)?,
                ))
            },
        )
        .optional()
        .map_err(StorageError::from)
    }

    pub fn load(&self, id: RepositoryId) -> Result<Option<RepositoryIndex>, StorageError> {
        let conn = self.lock()?;
        let repo = conn
            .query_row(
                "SELECT id, name, url, branch, commit_sha, line_count FROM repositories WHERE id = ?1",
                [id.to_string()],
                |row| Ok((row_repo(row)?, row.get::<_, i64>(5)? as u64)),
            )
            .optional()?;
        let Some((repository, line_count)) = repo else {
            return Ok(None);
        };

        let mut file_stmt = conn.prepare(
            "SELECT id, repository_id, path, language, size_bytes, hash FROM files WHERE repository_id = ?1",
        )?;
        let files = file_stmt
            .query_map([id.to_string()], |row| {
                Ok(SourceFile {
                    id: parse_id(row.get::<_, String>(0)?)?,
                    repository_id: parse_id(row.get::<_, String>(1)?)?,
                    path: row.get(2)?,
                    language: parse_enum::<Language, _>(row.get::<_, String>(3)?, StorageError::Language)?,
                    size_bytes: row.get::<_, i64>(4)? as u64,
                    hash: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut symbol_stmt = conn.prepare(
            "SELECT s.id, s.file_id, s.name, s.kind, s.start_line, s.end_line
             FROM symbols s JOIN files f ON f.id = s.file_id WHERE f.repository_id = ?1",
        )?;
        let symbols = symbol_stmt
            .query_map([id.to_string()], |row| {
                Ok(Symbol {
                    id: parse_id(row.get::<_, String>(0)?)?,
                    file_id: parse_id(row.get::<_, String>(1)?)?,
                    name: row.get(2)?,
                    kind: parse_enum::<SymbolKind, _>(row.get::<_, String>(3)?, StorageError::SymbolKind)?,
                    start_line: row.get::<_, i64>(4)? as u32,
                    end_line: row.get::<_, i64>(5)? as u32,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut rel_stmt = conn.prepare(
            "SELECT source_id, target_id, kind FROM relationships WHERE repository_id = ?1",
        )?;
        let relationships = rel_stmt
            .query_map([id.to_string()], |row| {
                Ok(Relationship {
                    source: parse_id(row.get::<_, String>(0)?)?,
                    target: parse_id(row.get::<_, String>(1)?)?,
                    kind: parse_enum::<RelationshipKind, _>(
                        row.get::<_, String>(2)?,
                        StorageError::RelationshipKind,
                    )?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut unit_stmt = conn.prepare(
            "SELECT c.symbol_id, f.repository_id, s.file_id, s.name, s.kind, f.path, f.language,
                    s.start_line, s.end_line, c.content
             FROM chunks c
             JOIN symbols s ON s.id = c.symbol_id
             JOIN files f ON f.id = s.file_id
             WHERE f.repository_id = ?1",
        )?;
        let units = unit_stmt
            .query_map([id.to_string()], |row| {
                Ok(SemanticUnit {
                    symbol_id: parse_id(row.get::<_, String>(0)?)?,
                    repository_id: parse_id(row.get::<_, String>(1)?)?,
                    file_id: parse_id(row.get::<_, String>(2)?)?,
                    name: row.get(3)?,
                    kind: parse_enum::<SymbolKind, _>(row.get::<_, String>(4)?, StorageError::SymbolKind)?,
                    path: row.get(5)?,
                    language: parse_enum::<Language, _>(row.get::<_, String>(6)?, StorageError::Language)?,
                    start_line: row.get::<_, i64>(7)? as u32,
                    end_line: row.get::<_, i64>(8)? as u32,
                    text: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

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
        let conn = self.lock()?;
        conn.execute(
            "INSERT OR REPLACE INTO git_evolution (repository_id, payload) VALUES (?1, ?2)",
            params![repository_id.to_string(), payload],
        )?;
        Ok(())
    }

    pub fn load_evolution(&self, repository_id: RepositoryId) -> Result<GitEvolution, StorageError> {
        let conn = self.lock()?;
        let payload = conn
            .query_row(
                "SELECT payload FROM git_evolution WHERE repository_id = ?1",
                [repository_id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        match payload {
            Some(payload) => serde_json::from_str(&payload)
                .map_err(|error| StorageError::Evolution(error.to_string())),
            None => Ok(GitEvolution::default()),
        }
    }

    pub fn save_run(&self, run: &AnalysisRun) -> Result<(), StorageError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO analysis_runs (
                id, repository_id, source, status, stage, files_discovered, files_parsed, files_skipped,
                symbols_extracted, relationships_built, chunks_embedded, message, error, started_at, finished_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(id) DO UPDATE SET
                repository_id = excluded.repository_id,
                status = excluded.status,
                stage = excluded.stage,
                files_discovered = excluded.files_discovered,
                files_parsed = excluded.files_parsed,
                files_skipped = excluded.files_skipped,
                symbols_extracted = excluded.symbols_extracted,
                relationships_built = excluded.relationships_built,
                chunks_embedded = excluded.chunks_embedded,
                message = excluded.message,
                error = excluded.error,
                finished_at = excluded.finished_at",
            params![
                run.id.to_string(),
                run.repository_id.map(|id| id.to_string()),
                run.source,
                run.status.as_str(),
                run.progress.stage.as_str(),
                run.progress.files_discovered as i64,
                run.progress.files_parsed as i64,
                run.progress.files_skipped as i64,
                run.progress.symbols_extracted as i64,
                run.progress.relationships_built as i64,
                run.progress.chunks_embedded as i64,
                run.progress.message,
                run.error,
                run.started_at,
                run.finished_at
            ],
        )?;
        Ok(())
    }

    pub fn load_run(&self, id: AnalysisRunId) -> Result<Option<AnalysisRun>, StorageError> {
        let conn = self.lock()?;
        conn.query_row(
            "SELECT id, repository_id, source, status, stage, files_discovered, files_parsed, files_skipped,
                    symbols_extracted, relationships_built, chunks_embedded, message, error, started_at, finished_at
             FROM analysis_runs WHERE id = ?1",
            [id.to_string()],
            row_run,
        )
        .optional()
        .map_err(StorageError::from)
    }

    pub fn search_symbols(&self, id: RepositoryId, query: &str) -> Result<Vec<Symbol>, StorageError> {
        let conn = self.lock()?;
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT s.id, s.file_id, s.name, s.kind, s.start_line, s.end_line
             FROM symbols s JOIN files f ON f.id = s.file_id
             WHERE f.repository_id = ?1 AND LOWER(s.name) LIKE LOWER(?2)
             ORDER BY s.name LIMIT 200",
        )?;
        let symbols = stmt
            .query_map(params![id.to_string(), pattern], |row| {
                Ok(Symbol {
                    id: parse_id(row.get::<_, String>(0)?)?,
                    file_id: parse_id(row.get::<_, String>(1)?)?,
                    name: row.get(2)?,
                    kind: parse_enum::<SymbolKind, _>(row.get::<_, String>(3)?, StorageError::SymbolKind)?,
                    start_line: row.get::<_, i64>(4)? as u32,
                    end_line: row.get::<_, i64>(5)? as u32,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(symbols)
    }
}

fn row_repo(row: &rusqlite::Row<'_>) -> rusqlite::Result<Repository> {
    Ok(Repository {
        id: parse_id(row.get::<_, String>(0)?)?,
        name: row.get(1)?,
        url: row.get(2)?,
        default_branch: row.get(3)?,
        commit_sha: row.get(4)?,
    })
}

fn row_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<AnalysisRun> {
    let repository_id = row
        .get::<_, Option<String>>(1)?
        .map(parse_id)
        .transpose()?;
    let status: String = row.get(3)?;
    let stage: String = row.get(4)?;
    Ok(AnalysisRun {
        id: parse_id(row.get::<_, String>(0)?)?,
        repository_id,
        source: row.get(2)?,
        status: status
            .parse::<IndexStage>()
            .map_err(|error| rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(StorageError::Run(error))))?,
        progress: IndexProgress {
            stage: stage
                .parse::<IndexStage>()
                .map_err(|error| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(StorageError::Run(error))))?,
            files_discovered: row.get::<_, i64>(5)? as u64,
            files_parsed: row.get::<_, i64>(6)? as u64,
            files_skipped: row.get::<_, i64>(7)? as u64,
            symbols_extracted: row.get::<_, i64>(8)? as u64,
            relationships_built: row.get::<_, i64>(9)? as u64,
            chunks_embedded: row.get::<_, i64>(10)? as u64,
            message: row.get(11)?,
            duration_ms: 0,
        },
        error: row.get(12)?,
        started_at: row.get(13)?,
        finished_at: row.get(14)?,
    })
}

fn migrate(conn: &Connection) -> Result<(), StorageError> {
    let columns = table_columns(conn, "repositories")?;
    if columns.iter().any(|col| col == "default_branch") && !columns.iter().any(|col| col == "branch") {
        conn.execute_batch("ALTER TABLE repositories RENAME COLUMN default_branch TO branch;")?;
    }
    if !table_columns(conn, "repositories")?
        .iter()
        .any(|col| col == "created_at")
    {
        conn.execute(
            "ALTER TABLE repositories ADD COLUMN created_at INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    let rels = table_columns(conn, "relationships")?;
    if rels.iter().any(|col| col == "source") && !rels.iter().any(|col| col == "source_id") {
        conn.execute_batch(
            "ALTER TABLE relationships RENAME COLUMN source TO source_id;
             ALTER TABLE relationships RENAME COLUMN target TO target_id;",
        )?;
    }
    let embeddings = table_columns(conn, "embeddings")?;
    if embeddings.iter().any(|col| col == "unit_id") && !embeddings.iter().any(|col| col == "chunk_id") {
        conn.execute_batch("ALTER TABLE embeddings RENAME COLUMN unit_id TO chunk_id;")?;
    }
    if table_exists(conn, "semantic_units")? {
        conn.execute_batch(
            "INSERT OR IGNORE INTO chunks (id, symbol_id, content, token_count)
             SELECT symbol_id, symbol_id, text, 0 FROM semantic_units;",
        )?;
    }
    Ok(())
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool, StorageError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>, StorageError> {
    if !table_exists(conn, table)? {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(StorageError::from)
}
