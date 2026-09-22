use std::path::Path;

use codeatlas_domain::{
    AnalysisRun, AnalysisRunId, GitEvolution, Repository, RepositoryId, RepositoryIndex, Symbol,
    SymbolId,
};

use crate::error::StorageError;
use crate::postgres::PostgresStore;
use crate::sqlite::SqliteStore;

/// Designed store is PostgreSQL. SQLite is the local/test stand-in with the same schema.
pub enum Store {
    Postgres(PostgresStore),
    Sqlite(SqliteStore),
}

impl Store {
    pub fn open(sqlite_path: &Path) -> Result<Self, StorageError> {
        match std::env::var("CODEATLAS_DATABASE_URL") {
            Ok(url) if url.starts_with("postgres://") || url.starts_with("postgresql://") => {
                Ok(Self::Postgres(PostgresStore::connect(&url)?))
            }
            _ => Ok(Self::Sqlite(SqliteStore::open(sqlite_path)?)),
        }
    }

    pub fn save(&self, index: &RepositoryIndex) -> Result<(), StorageError> {
        match self {
            Self::Postgres(store) => store.save(index),
            Self::Sqlite(store) => store.save(index),
        }
    }

    pub fn load(&self, id: RepositoryId) -> Result<Option<RepositoryIndex>, StorageError> {
        match self {
            Self::Postgres(store) => store.load(id),
            Self::Sqlite(store) => store.load(id),
        }
    }

    pub fn list_repositories(&self) -> Result<Vec<(Repository, u64)>, StorageError> {
        match self {
            Self::Postgres(store) => store.list_repositories(),
            Self::Sqlite(store) => store.list_repositories(),
        }
    }

    pub fn find_by_url(&self, url: &str) -> Result<Option<RepositoryId>, StorageError> {
        match self {
            Self::Postgres(store) => store.find_by_url(url),
            Self::Sqlite(store) => store.find_by_url(url),
        }
    }

    pub fn find_symbol(
        &self,
        id: SymbolId,
    ) -> Result<Option<(RepositoryId, Symbol, String)>, StorageError> {
        match self {
            Self::Postgres(store) => store.find_symbol(id),
            Self::Sqlite(store) => store.find_symbol(id),
        }
    }

    pub fn save_embeddings(
        &self,
        repository_id: RepositoryId,
        embeddings: &[(SymbolId, Vec<f32>)],
    ) -> Result<(), StorageError> {
        match self {
            Self::Postgres(store) => store.save_embeddings(repository_id, embeddings),
            Self::Sqlite(store) => store.save_embeddings(repository_id, embeddings),
        }
    }

    pub fn load_embeddings(
        &self,
        repository_id: RepositoryId,
    ) -> Result<Vec<(SymbolId, Vec<f32>)>, StorageError> {
        match self {
            Self::Postgres(store) => store.load_embeddings(repository_id),
            Self::Sqlite(store) => store.load_embeddings(repository_id),
        }
    }

    pub fn save_evolution(
        &self,
        repository_id: RepositoryId,
        evolution: &GitEvolution,
    ) -> Result<(), StorageError> {
        match self {
            Self::Postgres(store) => store.save_evolution(repository_id, evolution),
            Self::Sqlite(store) => store.save_evolution(repository_id, evolution),
        }
    }

    pub fn load_evolution(&self, repository_id: RepositoryId) -> Result<GitEvolution, StorageError> {
        match self {
            Self::Postgres(store) => store.load_evolution(repository_id),
            Self::Sqlite(store) => store.load_evolution(repository_id),
        }
    }

    pub fn save_run(&self, run: &AnalysisRun) -> Result<(), StorageError> {
        match self {
            Self::Postgres(store) => store.save_run(run),
            Self::Sqlite(store) => store.save_run(run),
        }
    }

    pub fn load_run(&self, id: AnalysisRunId) -> Result<Option<AnalysisRun>, StorageError> {
        match self {
            Self::Postgres(store) => store.load_run(id),
            Self::Sqlite(store) => store.load_run(id),
        }
    }

    pub fn search_symbols(&self, id: RepositoryId, query: &str) -> Result<Vec<Symbol>, StorageError> {
        match self {
            Self::Postgres(store) => store.search_symbols(id, query),
            Self::Sqlite(store) => store.search_symbols(id, query),
        }
    }
}
