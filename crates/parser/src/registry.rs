use std::path::Path;

use codeatlas_domain::Language;

use crate::error::ParseError;
use crate::languages::javascript::JavaScriptParser;
use crate::languages::python::PythonParser;
use crate::languages::rust::RustParser;
use crate::languages::typescript::TypeScriptParser;
use crate::parsed::ParsedFile;
use crate::LanguageParser;

enum RegisteredParser {
    TypeScript(TypeScriptParser),
    JavaScript(JavaScriptParser),
    Rust(RustParser),
    Python(PythonParser),
}

impl LanguageParser for RegisteredParser {
    fn language(&self) -> Language {
        match self {
            Self::TypeScript(parser) => parser.language(),
            Self::JavaScript(parser) => parser.language(),
            Self::Rust(parser) => parser.language(),
            Self::Python(parser) => parser.language(),
        }
    }

    fn parse(&self, source: &str) -> Result<ParsedFile, ParseError> {
        match self {
            Self::TypeScript(parser) => parser.parse(source),
            Self::JavaScript(parser) => parser.parse(source),
            Self::Rust(parser) => parser.parse(source),
            Self::Python(parser) => parser.parse(source),
        }
    }
}

pub struct ParserRegistry {
    typescript: RegisteredParser,
    tsx: RegisteredParser,
    javascript: RegisteredParser,
    jsx: RegisteredParser,
    rust: RegisteredParser,
    python: RegisteredParser,
}

impl ParserRegistry {
    pub fn new() -> Self {
        Self {
            typescript: RegisteredParser::TypeScript(TypeScriptParser::typescript()),
            tsx: RegisteredParser::TypeScript(TypeScriptParser::tsx()),
            javascript: RegisteredParser::JavaScript(JavaScriptParser::javascript()),
            jsx: RegisteredParser::JavaScript(JavaScriptParser::jsx()),
            rust: RegisteredParser::Rust(RustParser),
            python: RegisteredParser::Python(PythonParser),
        }
    }

    pub fn parser_for_path(&self, path: &str) -> Option<&dyn LanguageParser> {
        let ext = Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())?;

        let parser = match ext.as_str() {
            "ts" | "mts" | "cts" => &self.typescript,
            "tsx" => &self.tsx,
            "js" | "mjs" | "cjs" => &self.javascript,
            "jsx" => &self.jsx,
            "rs" => &self.rust,
            "py" | "pyi" => &self.python,
            _ => return None,
        };
        Some(parser)
    }

    pub fn parse_path(&self, path: &str, source: &str) -> Result<ParsedFile, ParseError> {
        self.parser_for_path(path)
            .ok_or(ParseError::UnsupportedLanguage)?
            .parse(source)
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}
