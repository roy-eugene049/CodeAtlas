use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use codeatlas_parser::ParserRegistry;

#[path = "support.rs"]
mod support;

fn parser(c: &mut Criterion) {
    let registry = ParserRegistry::new();
    let source = "export function login(user: string): boolean {\n  return user.length > 0;\n}\n";
    let mut group = c.benchmark_group("parser");
    for files in support::bench_sizes() {
        let bytes = (source.len() * files) as u64;
        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(BenchmarkId::from_parameter(files), &files, |bench, files| {
            bench.iter(|| {
                for index in 0..*files {
                    registry
                        .parse_path(&format!("src/f{index}.ts"), source)
                        .expect("parse");
                }
            });
        });
    }
    group.finish();
}

criterion_group!(benches, parser);
criterion_main!(benches);
