//! Persistence for repositories, files, symbols, relationships, chunks, embeddings, and analysis runs.
//!
//! PostgreSQL is the designed backend. SQLite uses the same table shapes for local work.

mod codec;
mod error;
mod postgres;
mod schema;
mod sqlite;
mod store;

pub use error::StorageError;
pub use postgres::PostgresStore;
pub use sqlite::SqliteStore;
pub use store::Store;

#[cfg(test)]
mod tests {
    use codeatlas_domain::{
        AnalysisRun, GitEvolution, Language, Repository, RepositoryIndex, SourceFile, Symbol,
        SymbolKind,
    };

    use super::*;

    #[test]
    fn round_trips_an_index_and_analysis_run() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::Sqlite(SqliteStore::open(&dir.path().join("index.db")).expect("open"));
        let repo = Repository::new("demo", "https://example.com/demo", "main", "sha");
        let file = SourceFile::new(repo.id, "src/lib.rs", Language::Rust, 32, "hash");
        let symbol = Symbol::new(file.id, "bootstrap", SymbolKind::Function, 1, 4);
        let index = RepositoryIndex {
            repository: repo.clone(),
            files: vec![file],
            symbols: vec![symbol.clone()],
            relationships: vec![],
            units: vec![],
            line_count: 4,
        };

        store.save(&index).expect("save");
        let loaded = store.load(repo.id).expect("load").expect("present");
        assert_eq!(loaded.repository.name, "demo");
        assert_eq!(loaded.symbols[0].name, "bootstrap");
        assert_eq!(store.find_by_url(&repo.url).expect("url"), Some(repo.id));
        let found = store.find_symbol(symbol.id).expect("find").expect("symbol");
        assert_eq!(found.0, repo.id);
        assert_eq!(found.2, "src/lib.rs");
        let hits = store.search_symbols(repo.id, "boot").expect("search");
        assert_eq!(hits.len(), 1);
        let evolution = GitEvolution::assemble(
            vec![codeatlas_domain::CommitTouch {
                sha: "aaa".into(),
                author_name: "Ada".into(),
                author_email: "ada@ex.com".into(),
                authored_at: 1,
                subject: "init".into(),
                paths: vec!["src/lib.rs".into()],
            }],
            vec![],
        );
        store.save_evolution(repo.id, &evolution).expect("save evo");
        let loaded_evo = store.load_evolution(repo.id).expect("load evo");
        assert_eq!(loaded_evo.hotspots[0].change_count, 1);

        let mut run = AnalysisRun::new("/tmp/demo");
        run.repository_id = Some(repo.id);
        store.save_run(&run).expect("save run");
        let loaded_run = store.load_run(run.id).expect("load run").expect("run");
        assert_eq!(loaded_run.source, "/tmp/demo");
        assert_eq!(loaded_run.repository_id, Some(repo.id));
    }
}
