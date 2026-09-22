use std::sync::Arc;

use codeatlas_api::service::RepositoryService;
use codeatlas_storage::Store;

#[test]
fn ask_returns_locations_confidence_and_file_ids() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::Sqlite(
        codeatlas_storage::SqliteStore::open(&dir.path().join("index.db")).expect("open"),
    );
    let service = RepositoryService::new(Arc::new(store), dir.path().join("cache"));
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../indexer/tests/fixtures/mini-repo");
    let (overview, id) = service
        .run_index(root.to_str().expect("utf8"), |_| {})
        .expect("index");
    assert!(overview.health.symbol_count > 0);

    let answer = service
        .ask(id, "How does authentication work?")
        .expect("ask");
    assert!(!answer.retrieval_summary.is_empty());
    assert!(answer.retrieval_summary.contains("relevant symbols"));
    assert!(!answer.citations.is_empty());
    assert!(answer
        .citations
        .iter()
        .any(|citation| !citation.path.is_empty() && citation.start_line > 0));
    assert!(
        answer.answer.contains(':') && answer.citations.iter().any(|citation| {
            answer.answer.contains(&format!(
                "{}:{}-{}",
                citation.path, citation.start_line, citation.end_line
            ))
        }),
        "answer should contain rewritten locations, got {}",
        answer.answer
    );
}
