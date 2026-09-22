use std::fs;
use std::path::Path;

use codeatlas_domain::SymbolKind;
use codeatlas_parser::ParserRegistry;

fn parse_tree(root: &Path) -> Vec<(String, codeatlas_parser::ParsedFile)> {
    let registry = ParserRegistry::new();
    let mut parsed = Vec::new();
    for entry in walkdir(root) {
        let relative = entry
            .strip_prefix(root)
            .expect("prefix")
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&entry).expect("read");
        let file = registry.parse_path(&relative, &source).expect("parse");
        parsed.push((relative, file));
    }
    parsed
}

fn walkdir(root: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    fn visit(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
        for entry in fs::read_dir(dir).expect("read_dir") {
            let entry = entry.expect("entry");
            let path = entry.path();
            if path.is_dir() {
                visit(&path, files);
            } else {
                files.push(path);
            }
        }
    }
    visit(root, &mut files);
    files.sort();
    files
}

fn has_symbol(parsed: &[(String, codeatlas_parser::ParsedFile)], name: &str, kind: SymbolKind) -> bool {
    parsed
        .iter()
        .any(|(_, file)| file.symbols.iter().any(|symbol| symbol.name == name && symbol.kind == kind))
}

fn has_call(parsed: &[(String, codeatlas_parser::ParsedFile)], callee: &str) -> bool {
    parsed
        .iter()
        .any(|(_, file)| file.calls.iter().any(|call| call.callee == callee))
}

#[test]
fn react_app_extracts_component_and_hook() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/projects/react-app");
    let parsed = parse_tree(&root);
    assert!(has_symbol(&parsed, "App", SymbolKind::Component));
    assert!(has_symbol(&parsed, "useAuth", SymbolKind::Function));
    assert!(has_call(&parsed, "useAuth"));
    assert!(parsed.iter().any(|(_, file)| {
        file.imports.iter().any(|import| import.module.contains("useAuth"))
    }));
}

#[test]
fn rust_project_extracts_service_and_bootstrap() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/projects/rust-project");
    let parsed = parse_tree(&root);
    assert!(has_symbol(&parsed, "AuthService", SymbolKind::Struct));
    assert!(has_symbol(&parsed, "login", SymbolKind::Method));
    assert!(has_symbol(&parsed, "bootstrap", SymbolKind::Function));
    assert!(has_call(&parsed, "login"));
}

#[test]
fn python_project_extracts_class_and_method() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/projects/python-project");
    let parsed = parse_tree(&root);
    assert!(has_symbol(&parsed, "AuthService", SymbolKind::Class));
    assert!(has_symbol(&parsed, "login", SymbolKind::Method));
    assert!(has_symbol(&parsed, "bootstrap", SymbolKind::Function));
    assert!(has_call(&parsed, "login"));
}

#[test]
fn mixed_project_extracts_rust_and_tsx() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/projects/mixed-project");
    let parsed = parse_tree(&root);
    assert!(has_symbol(&parsed, "serve", SymbolKind::Function));
    assert!(has_symbol(&parsed, "Checkout", SymbolKind::Component));
}
