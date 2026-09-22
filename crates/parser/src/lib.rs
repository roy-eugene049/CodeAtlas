//! Source → tree-sitter → syntax tree → normalized `ParsedFile`.
//!
//! Domain types stay language-agnostic. Tree-sitter stays inside this crate.

mod error;
mod languages;
mod parsed;
mod registry;
mod walk;

pub use error::ParseError;
pub use parsed::{ExtractedCall, ExtractedHeritage, ExtractedImport, ExtractedSymbol, ParsedFile};
pub use registry::ParserRegistry;

use codeatlas_domain::Language;

pub trait LanguageParser: Send + Sync {
    fn language(&self) -> Language;
    fn parse(&self, source: &str) -> Result<ParsedFile, ParseError>;
}

#[cfg(test)]
mod tests {
    use codeatlas_domain::{RelationshipKind, SymbolKind};

    use super::*;
    use crate::languages::javascript::JavaScriptParser;
    use crate::languages::python::PythonParser;
    use crate::languages::rust::RustParser;
    use crate::languages::typescript::TypeScriptParser;

    fn has_symbol(parsed: &ParsedFile, name: &str, kind: SymbolKind) -> bool {
        parsed
            .symbols
            .iter()
            .any(|symbol| symbol.name == name && symbol.kind == kind)
    }

    #[test]
    fn rust_parser_extracts_core_symbols() {
        let source = include_str!("../fixtures/rust/sample.rs");
        let parsed = RustParser.parse(source).expect("parse rust");

        assert!(has_symbol(&parsed, "AuthService", SymbolKind::Struct));
        assert!(has_symbol(&parsed, "Repository", SymbolKind::Trait));
        assert!(has_symbol(&parsed, "login", SymbolKind::Method));
        assert!(has_symbol(&parsed, "bootstrap", SymbolKind::Function));
        assert!(parsed.imports.iter().any(|import| import.names.contains(&"UserRepository".to_string())));
        assert!(parsed.calls.iter().any(|call| call.callee == "login"));
        assert!(parsed.heritage.iter().any(|edge| {
            edge.child == "AuthService"
                && edge.parent == "Repository"
                && edge.kind == RelationshipKind::Implements
        }));
    }

    #[test]
    fn typescript_parser_extracts_interfaces_and_components() {
        let source = include_str!("../fixtures/typescript/sample.ts");
        let parsed = TypeScriptParser::typescript()
            .parse(source)
            .expect("parse typescript");

        assert!(has_symbol(&parsed, "User", SymbolKind::Interface));
        assert!(has_symbol(&parsed, "AuthService", SymbolKind::Class));
        assert!(has_symbol(&parsed, "login", SymbolKind::Method));
        assert!(parsed.imports.iter().any(|import| import.module.contains("repository")));
    }

    #[test]
    fn tsx_parser_marks_pascal_case_functions_as_components() {
        let source = include_str!("../fixtures/typescript/component.tsx");
        let parsed = TypeScriptParser::tsx().parse(source).expect("parse tsx");

        assert!(has_symbol(&parsed, "AuthPage", SymbolKind::Component));
        assert!(has_symbol(&parsed, "useSession", SymbolKind::Function));
        assert!(parsed.calls.iter().any(|call| call.callee == "useAuth"));
    }

    #[test]
    fn javascript_parser_extracts_functions_and_imports() {
        let source = include_str!("../fixtures/javascript/sample.js");
        let parsed = JavaScriptParser::javascript()
            .parse(source)
            .expect("parse javascript");

        assert!(has_symbol(&parsed, "createClient", SymbolKind::Function));
        assert!(has_symbol(&parsed, "MAX_RETRIES", SymbolKind::Constant));
        assert!(parsed.imports.iter().any(|import| import.module == "./http"));
    }

    #[test]
    fn python_parser_extracts_classes_and_heritage() {
        let source = include_str!("../fixtures/python/sample.py");
        let parsed = PythonParser.parse(source).expect("parse python");

        assert!(has_symbol(&parsed, "AuthService", SymbolKind::Class));
        assert!(has_symbol(&parsed, "login", SymbolKind::Method));
        assert!(has_symbol(&parsed, "TIMEOUT", SymbolKind::Constant));
        assert!(parsed.heritage.iter().any(|edge| edge.child == "AuthService" && edge.parent == "Service"));
        assert!(parsed.imports.iter().any(|import| import.module == "db.repository"));
    }

    #[test]
    fn registry_selects_parser_by_path() {
        let registry = ParserRegistry::new();
        let rust = include_str!("../fixtures/rust/sample.rs");
        let parsed = registry.parse_path("src/auth.rs", rust).expect("registry rust");
        assert!(has_symbol(&parsed, "AuthService", SymbolKind::Struct));
        assert!(registry.parse_path("notes.md", "hello").is_err());
    }
}
