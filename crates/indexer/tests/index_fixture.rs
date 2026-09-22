use codeatlas_domain::SymbolKind;
use codeatlas_indexer::{materialize_local, Indexer};

#[test]
fn indexes_mini_repo_into_a_symbol_graph() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini-repo");
    let source = materialize_local(&root).expect("materialize fixture");
    let index = Indexer::default().index(&source).expect("index fixture");

    assert!(index.files.len() >= 4);
    assert!(index.symbols.iter().any(|symbol| {
        symbol.name == "AuthService" && symbol.kind == SymbolKind::Class
    }));
    assert!(index.symbols.iter().any(|symbol| {
        symbol.name == "AuthController" && symbol.kind == SymbolKind::Function
    }));
    assert!(index.symbols.iter().any(|symbol| {
        symbol.name == "useAuth" && symbol.kind == SymbolKind::Function
    }));
    assert!(index.symbols.iter().any(|symbol| {
        symbol.name == "UserRepository" && symbol.kind == SymbolKind::Class
    }));
    assert!(!index.relationships.is_empty());
    assert!(index
        .relationships
        .iter()
        .any(|rel| rel.kind == codeatlas_domain::RelationshipKind::Imports));
    let login = index
        .symbols
        .iter()
        .find(|symbol| symbol.name == "login")
        .expect("login method");
    let login_callers = index
        .relationships
        .iter()
        .filter(|rel| {
            rel.target == login.id && rel.kind == codeatlas_domain::RelationshipKind::Calls
        })
        .count();
    assert!(
        login_callers > 0,
        "useAuth should call AuthService.login, got {login_callers} callers"
    );
    assert!(index.line_count > 0);
    assert!(
        index
            .units
            .iter()
            .any(|unit| unit.name == "handleFailure" && unit.text.contains("retryPayment")),
        "each symbol should become its own semantic unit"
    );
    assert!(
        index
            .units
            .iter()
            .all(|unit| unit.text.lines().count() <= 80),
        "units must not embed entire files"
    );
    assert!(
        index.symbols.iter().any(|symbol| symbol.name == "handlePayment"),
        "payment fixture should expose handlePayment"
    );
}

#[test]
fn incremental_index_skips_unchanged_hashes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).expect("src");
    std::fs::write(src.join("keep.ts"), "export function keep() { return 1; }\n").expect("keep");
    std::fs::write(src.join("touch.ts"), "export function touch() { return 1; }\n").expect("touch");

    let source = materialize_local(dir.path()).expect("materialize");
    let indexer = Indexer::default();
    let first = indexer.index(&source).expect("first index");
    let first_keep = first
        .symbols
        .iter()
        .find(|symbol| symbol.name == "keep")
        .expect("keep")
        .id;

    std::fs::write(src.join("touch.ts"), "export function touch() { return 2; }\n").expect("rewrite");
    let second = indexer
        .index_with_previous(&source, Some(&first), |_| {})
        .expect("second index");

    assert_eq!(second.progress.files_skipped, 1);
    assert_eq!(second.progress.files_parsed, 1);
    let second_keep = second
        .index
        .symbols
        .iter()
        .find(|symbol| symbol.name == "keep")
        .expect("keep after increment")
        .id;
    assert_eq!(first_keep, second_keep);
    assert!(second
        .changed_symbol_ids
        .iter()
        .any(|id| second.index.symbols.iter().any(|symbol| symbol.id == *id && symbol.name == "touch")));
}

#[test]
fn git_commit_range_only_reparses_affected_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    run(root, &["git", "init", "-b", "main"]);
    run(root, &["git", "config", "user.name", "Ada"]);
    run(root, &["git", "config", "user.email", "ada@ex.com"]);
    write(root, "src/keep.ts", "export function keep() { return 1; }\n");
    write(root, "src/touch.ts", "export function touch() { return 1; }\n");
    write(root, "src/gone.ts", "export function gone() { return 1; }\n");
    run(root, &["git", "add", "."]);
    run(root, &["git", "commit", "-m", "A"]);

    let source = materialize_local(root).expect("materialize");
    let indexer = Indexer::default();
    let first = indexer.index(&source).expect("first");
    assert!(first.symbols.iter().any(|symbol| symbol.name == "gone"));

    std::fs::write(root.join("src/touch.ts"), "export function touch() { return 2; }\n").expect("edit");
    std::fs::remove_file(root.join("src/gone.ts")).expect("delete");
    write(root, "src/new.ts", "export function created() { return 1; }\n");
    run(root, &["git", "add", "-A"]);
    run(root, &["git", "commit", "-m", "B"]);

    let source = materialize_local(root).expect("materialize b");
    let second = indexer
        .index_with_previous(&source, Some(&first), |_| {})
        .expect("second");
    assert_eq!(second.progress.files_parsed, 2, "touch + new");
    assert!(second.progress.files_skipped >= 1);
    assert!(second.index.symbols.iter().any(|symbol| symbol.name == "created"));
    assert!(second.index.symbols.iter().any(|symbol| symbol.name == "keep"));
    assert!(!second.index.symbols.iter().any(|symbol| symbol.name == "gone"));
}

#[test]
fn refuses_to_index_env_and_ignored_trees() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "src/app.ts", "export const ok = 1;\n");
    write(dir.path(), ".env", "API_KEY=super-secret\n");
    write(dir.path(), "node_modules/pkg.ts", "export const skip = 1;\n");
    write(dir.path(), "target/out.rs", "pub fn skip() {}\n");
    let source = materialize_local(dir.path()).expect("materialize");
    let index = Indexer::default().index(&source).expect("index");
    assert_eq!(index.files.len(), 1);
    assert_eq!(index.files[0].path, "src/app.ts");
}

fn write(root: &std::path::Path, path: &str, body: &str) {
    let full = root.join(path);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).expect("mkdir");
    }
    std::fs::write(full, body).expect("write");
}

fn run(cwd: &std::path::Path, args: &[&str]) {
    let status = std::process::Command::new(args[0])
        .args(&args[1..])
        .current_dir(cwd)
        .status()
        .expect("spawn");
    assert!(status.success(), "{args:?}");
}
