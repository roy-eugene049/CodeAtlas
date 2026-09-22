use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

#[path = "support.rs"]
mod support;

fn indexing(c: &mut Criterion) {
    let mut group = c.benchmark_group("indexing");
    group.sample_size(10);
    for files in support::bench_sizes() {
        let dir = tempfile::tempdir().expect("tempdir");
        support::write_ts_repo(dir.path(), files);
        let bytes = dir_bytes(dir.path());
        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(BenchmarkId::from_parameter(files), &(), |bench, _| {
            bench.iter(|| support::index_repo(dir.path()));
        });
    }
    group.finish();
}

fn dir_bytes(root: &std::path::Path) -> u64 {
    let mut total = 0u64;
    fn visit(dir: &std::path::Path, total: &mut u64) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit(&path, total);
            } else if let Ok(meta) = entry.metadata() {
                *total += meta.len();
            }
        }
    }
    visit(root, &mut total);
    total
}

criterion_group!(benches, indexing);
criterion_main!(benches);
