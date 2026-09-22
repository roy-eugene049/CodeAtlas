use std::fs;
use std::path::{Path, PathBuf};

use codeatlas_domain::{
    Language, Relationship, RelationshipKind, Repository, RepositoryIndex, SemanticUnit, SourceFile,
    Symbol, SymbolKind,
};
use codeatlas_indexer::{materialize_local, Indexer};

pub fn bench_sizes() -> Vec<usize> {
    match std::env::var("CODEATLAS_BENCH_FILES") {
        Ok(value) => value
            .split(',')
            .filter_map(|part| part.trim().parse().ok())
            .filter(|count| *count > 0)
            .collect(),
        Err(_) => vec![256, 1_024],
    }
}

pub fn write_ts_repo(root: &Path, files: usize) {
    for index in 0..files {
        let dir = root.join("src").join(format!("m{index}"));
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(
            dir.join("mod.ts"),
            format!(
                "export function fn{index}(value: number): number {{\n  return helper{index}(value);\n}}\nexport function helper{index}(value: number): number {{\n  return value + {index};\n}}\n"
            ),
        )
        .expect("write");
    }
}

pub fn index_repo(root: &Path) -> RepositoryIndex {
    let source = materialize_local(root).expect("materialize");
    Indexer::default()
        .with_parse_threads(4)
        .index(&source)
        .expect("index")
}

pub fn synthetic_index(files: usize) -> (PathBuf, RepositoryIndex) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().to_path_buf();
    write_ts_repo(&root, files);
    let index = index_repo(&root);
    std::mem::forget(dir);
    (root, index)
}

pub fn graph_index(nodes: usize) -> RepositoryIndex {
    let repo = Repository::new("bench", "https://example.com/bench", "main", "sha");
    let mut files = Vec::new();
    let mut symbols = Vec::new();
    let mut relationships = Vec::new();
    let mut units = Vec::new();
    for index in 0..nodes {
        let file = SourceFile::new(
            repo.id,
            format!("src/f{index}.ts"),
            Language::TypeScript,
            40,
            format!("h{index}"),
        );
        let symbol = Symbol::new(
            file.id,
            format!("fn{index}"),
            SymbolKind::Function,
            1,
            8,
        );
        if index > 0 {
            relationships.push(Relationship::new(
                symbols[index - 1].id,
                symbol.id,
                RelationshipKind::Calls,
            ));
        }
        units.push(SemanticUnit {
            symbol_id: symbol.id,
            repository_id: repo.id,
            file_id: file.id,
            name: symbol.name.clone(),
            kind: symbol.kind,
            path: file.path.clone(),
            language: file.language,
            start_line: 1,
            end_line: 8,
            text: format!("export function fn{index}() {{ return {index}; }}"),
        });
        files.push(file);
        symbols.push(symbol);
    }
    RepositoryIndex {
        repository: repo,
        files,
        symbols,
        relationships,
        units,
        line_count: nodes as u64 * 8,
    }
}
