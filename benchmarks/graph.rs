use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use codeatlas_graph::CodeGraph;

#[path = "support.rs"]
mod support;

fn graph(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph");
    for nodes in support::bench_sizes() {
        let index = support::graph_index(nodes);
        group.bench_with_input(BenchmarkId::from_parameter(nodes), &index, |bench, index| {
            bench.iter(|| {
                let graph = CodeGraph::build(index);
                if let Some(first) = index.symbols.first() {
                    let _ = graph.impact(first.id);
                    let _ = graph.neighborhood(first.id, 2);
                }
            });
        });
    }
    group.finish();
}

criterion_group!(benches, graph);
criterion_main!(benches);
