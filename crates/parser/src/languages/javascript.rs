use codeatlas_domain::Language;
use tree_sitter::Parser;

use crate::error::ParseError;
use crate::languages::ecma::extract_ecma;
use crate::parsed::ParsedFile;
use crate::LanguageParser;

pub struct JavaScriptParser {
    jsx: bool,
}

impl JavaScriptParser {
    pub fn javascript() -> Self {
        Self { jsx: false }
    }

    pub fn jsx() -> Self {
        Self { jsx: true }
    }
}

impl LanguageParser for JavaScriptParser {
    fn language(&self) -> Language {
        Language::JavaScript
    }

    fn parse(&self, source: &str) -> Result<ParsedFile, ParseError> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_javascript::LANGUAGE.into())
            .map_err(|error| ParseError::Language(error.to_string()))?;
        let tree = parser.parse(source, None).ok_or(ParseError::ParseFailed)?;
        Ok(extract_ecma(tree.root_node(), source, self.jsx))
    }
}
