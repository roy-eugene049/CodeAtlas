use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use codeatlas_search::{exact_search, embed_units, HashedEmbedder, HashedSemanticSearch, SemanticSearch};

#[path = "support.rs"]
mod support;

fn search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");
    for nodes in support::bench_sizes() {
        let index = support::graph_index(nodes);
        let embeddings = embed_units(&HashedEmbedder::default(), &index.units).expect("embed");
        let semantic = HashedSemanticSearch::new(&index.units, &embeddings);
        group.bench_with_input(BenchmarkId::from_parameter(nodes), &(), |bench, _| {
            bench.iter(|| {
                let _ = exact_search(&index, "fn12", 16);
                let _ = SemanticSearch::search(&semantic, "function helper", 8).expect("search");
            });
        });
    }
    group.finish();
}

criterion_group!(benches, search);
criterion_main!(benches);
