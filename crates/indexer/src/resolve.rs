use std::collections::HashMap;
use std::path::Path;

use codeatlas_domain::{
    FileId, Relationship, RelationshipKind, SourceFile, Symbol, SymbolId, SymbolKind,
};
use codeatlas_parser::ParsedFile;

#[derive(Debug, Default)]
pub struct ResolvedGraph {
    pub symbols: Vec<Symbol>,
    pub relationships: Vec<Relationship>,
}

pub fn resolve(files: &[(SourceFile, ParsedFile)]) -> ResolvedGraph {
    let mut symbols = Vec::new();
    let mut file_symbols: HashMap<FileId, Vec<Symbol>> = HashMap::new();

    for (file, parsed) in files {
        for extracted in &parsed.symbols {
            let symbol = Symbol::new(
                file.id,
                extracted.name.clone(),
                extracted.kind,
                extracted.start_line,
                extracted.end_line,
            );
            file_symbols.entry(file.id).or_default().push(symbol.clone());
            symbols.push(symbol);
        }
    }

    let mut by_file_name: HashMap<(FileId, String), Vec<SymbolId>> = HashMap::new();
    for symbol in &symbols {
        by_file_name
            .entry((symbol.file_id, symbol.name.clone()))
            .or_default()
            .push(symbol.id);
    }

    let mut path_to_file: HashMap<String, FileId> = HashMap::new();
    for (file, _) in files {
        path_to_file.insert(file.path.clone(), file.id);
    }

    let mut relationships = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for (file, parsed) in files {
        let locals = file_symbols.get(&file.id).cloned().unwrap_or_default();

        for extracted in &parsed.symbols {
            if let Some(container) = &extracted.container_name {
                if let Some(parent_id) = first_id(&by_file_name, file.id, container) {
                    if let Some(child_id) = first_id(&by_file_name, file.id, &extracted.name) {
                        push_rel(
                            &mut relationships,
                            &mut seen,
                            parent_id,
                            child_id,
                            RelationshipKind::Contains,
                        );
                    }
                }
            }
        }

        let import_targets = resolve_imports(file, parsed, &path_to_file, &file_symbols);
        let top_level: Vec<SymbolId> = locals
            .iter()
            .filter(|symbol| {
                !matches!(symbol.kind, SymbolKind::Method)
                    && parsed
                        .symbols
                        .iter()
                        .find(|extracted| extracted.name == symbol.name)
                        .and_then(|extracted| extracted.container_name.as_ref())
                        .is_none()
            })
            .map(|symbol| symbol.id)
            .collect();

        for target in &import_targets {
            for source in &top_level {
                push_rel(
                    &mut relationships,
                    &mut seen,
                    *source,
                    *target,
                    RelationshipKind::Imports,
                );
            }
        }

        for call in &parsed.calls {
            let Some(caller) = enclosing_symbol(&locals, call.line) else {
                continue;
            };
            if let Some(callee) = resolve_name(&call.callee, file, &locals, &import_targets, &file_symbols, &by_file_name)
            {
                if caller != callee {
                    push_rel(
                        &mut relationships,
                        &mut seen,
                        caller,
                        callee,
                        RelationshipKind::Calls,
                    );
                }
            }
        }

        for edge in &parsed.heritage {
            let Some(child) = first_id(&by_file_name, file.id, &edge.child) else {
                continue;
            };
            if let Some(parent) = resolve_name(
                &edge.parent,
                file,
                &locals,
                &import_targets,
                &file_symbols,
                &by_file_name,
            ) {
                push_rel(&mut relationships, &mut seen, child, parent, edge.kind);
            }
        }
    }

    ResolvedGraph {
        symbols,
        relationships,
    }
}

/// Resolve edges that originate in `changed` against the full live symbol table.
pub fn resolve_changed(
    live_symbols: &[Symbol],
    files: &[SourceFile],
    changed: &[(SourceFile, ParsedFile)],
) -> Vec<Relationship> {
    let mut file_symbols: HashMap<FileId, Vec<Symbol>> = HashMap::new();
    let mut by_file_name: HashMap<(FileId, String), Vec<SymbolId>> = HashMap::new();
    for symbol in live_symbols {
        file_symbols
            .entry(symbol.file_id)
            .or_default()
            .push(symbol.clone());
        by_file_name
            .entry((symbol.file_id, symbol.name.clone()))
            .or_default()
            .push(symbol.id);
    }
    let path_to_file = files
        .iter()
        .map(|file| (file.path.clone(), file.id))
        .collect::<HashMap<_, _>>();
    let mut relationships = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (file, parsed) in changed {
        let locals = file_symbols.get(&file.id).cloned().unwrap_or_default();
        for extracted in &parsed.symbols {
            if let Some(container) = &extracted.container_name {
                if let Some(parent_id) = first_id(&by_file_name, file.id, container) {
                    if let Some(child_id) = first_id(&by_file_name, file.id, &extracted.name) {
                        push_rel(
                            &mut relationships,
                            &mut seen,
                            parent_id,
                            child_id,
                            RelationshipKind::Contains,
                        );
                    }
                }
            }
        }
        let import_targets = resolve_imports(file, parsed, &path_to_file, &file_symbols);
        let top_level: Vec<SymbolId> = locals
            .iter()
            .filter(|symbol| {
                !matches!(symbol.kind, SymbolKind::Method)
                    && parsed
                        .symbols
                        .iter()
                        .find(|extracted| extracted.name == symbol.name)
                        .and_then(|extracted| extracted.container_name.as_ref())
                        .is_none()
            })
            .map(|symbol| symbol.id)
            .collect();
        for target in &import_targets {
            for source in &top_level {
                push_rel(
                    &mut relationships,
                    &mut seen,
                    *source,
                    *target,
                    RelationshipKind::Imports,
                );
            }
        }
        for call in &parsed.calls {
            let Some(caller) = enclosing_symbol(&locals, call.line) else {
                continue;
            };
            if let Some(callee) =
                resolve_name(&call.callee, file, &locals, &import_targets, &file_symbols, &by_file_name)
            {
                if caller != callee {
                    push_rel(
                        &mut relationships,
                        &mut seen,
                        caller,
                        callee,
                        RelationshipKind::Calls,
                    );
                }
            }
        }
        for edge in &parsed.heritage {
            let Some(child) = first_id(&by_file_name, file.id, &edge.child) else {
                continue;
            };
            if let Some(parent) = resolve_name(
                &edge.parent,
                file,
                &locals,
                &import_targets,
                &file_symbols,
                &by_file_name,
            ) {
                push_rel(&mut relationships, &mut seen, child, parent, edge.kind);
            }
        }
    }
    relationships
}

fn first_id(
    by_file_name: &HashMap<(FileId, String), Vec<SymbolId>>,
    file_id: FileId,
    name: &str,
) -> Option<SymbolId> {
    by_file_name
        .get(&(file_id, name.to_string()))
        .and_then(|ids| ids.first().copied())
}

fn enclosing_symbol(symbols: &[Symbol], line: u32) -> Option<SymbolId> {
    symbols
        .iter()
        .filter(|symbol| symbol.contains_line(line))
        .min_by_key(|symbol| symbol.end_line.saturating_sub(symbol.start_line))
        .map(|symbol| symbol.id)
}

fn resolve_imports(
    file: &SourceFile,
    parsed: &ParsedFile,
    path_to_file: &HashMap<String, FileId>,
    file_symbols: &HashMap<FileId, Vec<Symbol>>,
) -> Vec<SymbolId> {
    let mut targets = Vec::new();
    for import in &parsed.imports {
        let Some(target_file) = resolve_module(&file.path, &import.module, path_to_file) else {
            continue;
        };
        let Some(symbols) = file_symbols.get(&target_file) else {
            continue;
        };
        if import.names.is_empty() {
            targets.extend(symbols.iter().map(|symbol| symbol.id));
            continue;
        }
        for name in &import.names {
            if let Some(symbol) = symbols.iter().find(|symbol| &symbol.name == name) {
                targets.push(symbol.id);
            }
        }
    }
    targets
}

fn resolve_name(
    name: &str,
    file: &SourceFile,
    locals: &[Symbol],
    import_targets: &[SymbolId],
    file_symbols: &HashMap<FileId, Vec<Symbol>>,
    by_file_name: &HashMap<(FileId, String), Vec<SymbolId>>,
) -> Option<SymbolId> {
    if let Some(local) = locals.iter().find(|symbol| symbol.name == name) {
        return Some(local.id);
    }
    if let Some(id) = first_id(by_file_name, file.id, name) {
        return Some(id);
    }
    for target in import_targets {
        let Some(imported) = file_symbols.values().flatten().find(|symbol| symbol.id == *target) else {
            continue;
        };
        if imported.name == name {
            return Some(imported.id);
        }
        if let Some(member) = file_symbols
            .get(&imported.file_id)
            .and_then(|symbols| symbols.iter().find(|symbol| symbol.name == name))
        {
            return Some(member.id);
        }
    }
    None
}

fn resolve_module(
    from_path: &str,
    module: &str,
    path_to_file: &HashMap<String, FileId>,
) -> Option<FileId> {
    if !(module.starts_with('.') || module.starts_with('/')) {
        return None;
    }

    let from_dir = Path::new(from_path).parent().unwrap_or(Path::new(""));
    let joined = from_dir.join(module);
    let candidates = [
        joined.with_extension("ts"),
        joined.with_extension("tsx"),
        joined.with_extension("js"),
        joined.with_extension("jsx"),
        joined.with_extension("rs"),
        joined.with_extension("py"),
        joined.join("index.ts"),
        joined.join("index.js"),
        joined.join("mod.rs"),
        joined.join("__init__.py"),
        joined.clone(),
    ];

    for candidate in candidates {
        let key = normalize_path(&candidate);
        if let Some(id) = path_to_file.get(&key) {
            return Some(*id);
        }
    }
    None
}

fn normalize_path(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::Normal(part) => {
                parts.push(part.to_string_lossy().into_owned());
            }
            std::path::Component::RootDir => {}
            std::path::Component::CurDir => {}
            std::path::Component::Prefix(_) => {}
        }
    }
    parts.join("/")
}

fn push_rel(
    relationships: &mut Vec<Relationship>,
    seen: &mut std::collections::HashSet<(SymbolId, SymbolId, RelationshipKind)>,
    source: SymbolId,
    target: SymbolId,
    kind: RelationshipKind,
) {
    if source == target {
        return;
    }
    if seen.insert((source, target, kind)) {
        relationships.push(Relationship::new(source, target, kind));
    }
}
