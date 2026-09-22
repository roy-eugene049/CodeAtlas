use codeatlas_domain::Language;
use tree_sitter::Parser;

use crate::error::ParseError;
use crate::languages::ecma::extract_ecma;
use crate::parsed::ParsedFile;
use crate::LanguageParser;

pub struct TypeScriptParser {
    jsx: bool,
}

impl TypeScriptParser {
    pub fn typescript() -> Self {
        Self { jsx: false }
    }

    pub fn tsx() -> Self {
        Self { jsx: true }
    }
}

impl LanguageParser for TypeScriptParser {
    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn parse(&self, source: &str) -> Result<ParsedFile, ParseError> {
        let mut parser = Parser::new();
        let language = if self.jsx {
            tree_sitter_typescript::LANGUAGE_TSX
        } else {
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT
        };
        parser
            .set_language(&language.into())
            .map_err(|error| ParseError::Language(error.to_string()))?;
        let tree = parser.parse(source, None).ok_or(ParseError::ParseFailed)?;
        Ok(extract_ecma(tree.root_node(), source, self.jsx))
    }
}
