//! Wall-clock MVP timings. Print measured numbers; do not invent them.

use std::path::Path;
use std::time::Instant;

use codeatlas_ai::{embed_units, ContextEngine, ExtractiveModel, HashedEmbedder};
use codeatlas_graph::CodeGraph;
use codeatlas_indexer::{materialize_local, Indexer};
use codeatlas_search::exact_search;

fn main() {
    let indexer = Indexer::default();
    let roots = [
        (
            "mini-repo",
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../crates/indexer/tests/fixtures/mini-repo"),
        ),
        ("CodeAtlas", Path::new(env!("CARGO_MANIFEST_DIR")).join("..")),
    ];

    println!("Repository       Files   Symbols   Index     Search    Ask");
    println!("--------------------------------------------------------------");

    for (name, root) in roots {
        let source = materialize_local(&root).expect("materialize");
        let started = Instant::now();
        let index = indexer.index(&source).expect("index");
        let index_ms = started.elapsed();

        let embeddings = embed_units(&HashedEmbedder::default(), &index.units).expect("embed");
        let graph = CodeGraph::build(&index);
        if let Some(symbol) = index.symbols.first() {
            let _ = graph.impact(symbol.id);
        }

        let started = Instant::now();
        let _ = exact_search(&index, "authentication", 16);
        let search_ms = started.elapsed();

        let engine = ContextEngine::new(HashedEmbedder::default(), ExtractiveModel);
        let started = Instant::now();
        let _ = engine.ask("How does authentication work?", &index, &embeddings);
        let ask_ms = started.elapsed();

        println!(
            "{:<16} {:>5} {:>9} {:>7.2}s {:>7.1}ms {:>6.1}ms",
            name,
            index.files.len(),
            index.symbols.len(),
            index_ms.as_secs_f64(),
            search_ms.as_secs_f64() * 1000.0,
            ask_ms.as_secs_f64() * 1000.0,
        );
    }
}
