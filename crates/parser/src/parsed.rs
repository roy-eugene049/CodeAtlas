use codeatlas_domain::{RelationshipKind, SymbolKind};

/// Language-agnostic extract from a tree-sitter tree. The indexer consumes this,
/// not raw syntax nodes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedFile {
    pub symbols: Vec<ExtractedSymbol>,
    pub imports: Vec<ExtractedImport>,
    pub calls: Vec<ExtractedCall>,
    pub heritage: Vec<ExtractedHeritage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub start_line: u32,
    pub end_line: u32,
    pub container_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedImport {
    pub module: String,
    pub names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedCall {
    pub callee: String,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedHeritage {
    pub child: String,
    pub parent: String,
    pub kind: RelationshipKind,
}
