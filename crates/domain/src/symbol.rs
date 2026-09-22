use serde::{Deserialize, Serialize};

use crate::ids::{FileId, SymbolId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SymbolKind {
    Function,
    Class,
    Interface,
    Struct,
    Enum,
    Trait,
    Component,
    Variable,
    Constant,
    Method,
}

impl SymbolKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Class => "class",
            Self::Interface => "interface",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
            Self::Component => "component",
            Self::Variable => "variable",
            Self::Constant => "constant",
            Self::Method => "method",
        }
    }
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for SymbolKind {
    type Err = UnknownSymbolKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "function" => Ok(Self::Function),
            "class" => Ok(Self::Class),
            "interface" => Ok(Self::Interface),
            "struct" => Ok(Self::Struct),
            "enum" => Ok(Self::Enum),
            "trait" => Ok(Self::Trait),
            "component" => Ok(Self::Component),
            "variable" => Ok(Self::Variable),
            "constant" => Ok(Self::Constant),
            "method" => Ok(Self::Method),
            other => Err(UnknownSymbolKind(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownSymbolKind(pub String);

impl std::fmt::Display for UnknownSymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown symbol kind: {}", self.0)
    }
}

impl std::error::Error for UnknownSymbolKind {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol {
    pub id: SymbolId,
    pub file_id: FileId,
    pub name: String,
    pub kind: SymbolKind,
    pub start_line: u32,
    pub end_line: u32,
}

impl Symbol {
    pub fn new(
        file_id: FileId,
        name: impl Into<String>,
        kind: SymbolKind,
        start_line: u32,
        end_line: u32,
    ) -> Self {
        let name = name.into();
        Self {
            id: SymbolId::stable(file_id, &name, kind, start_line),
            file_id,
            name,
            kind,
            start_line,
            end_line,
        }
    }

    pub fn contains_line(&self, line: u32) -> bool {
        line >= self.start_line && line <= self.end_line
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::FileId;

    #[test]
    fn method_kind_round_trips() {
        assert_eq!(SymbolKind::Method.as_str().parse::<SymbolKind>().ok(), Some(SymbolKind::Method));
    }

    #[test]
    fn symbol_owns_its_line_span() {
        let symbol = Symbol::new(FileId::new(), "login", SymbolKind::Function, 10, 20);
        assert!(symbol.contains_line(10));
        assert!(symbol.contains_line(20));
        assert!(!symbol.contains_line(9));
        assert!(!symbol.contains_line(21));
    }
}
