use serde::{Deserialize, Serialize};

use crate::ids::SymbolId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RelationshipKind {
    Imports,
    Calls,
    Extends,
    Implements,
    References,
    Contains,
}

impl RelationshipKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Imports => "imports",
            Self::Calls => "calls",
            Self::Extends => "extends",
            Self::Implements => "implements",
            Self::References => "references",
            Self::Contains => "contains",
        }
    }
}

impl std::fmt::Display for RelationshipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for RelationshipKind {
    type Err = UnknownRelationshipKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "imports" => Ok(Self::Imports),
            "calls" => Ok(Self::Calls),
            "extends" => Ok(Self::Extends),
            "implements" => Ok(Self::Implements),
            "references" => Ok(Self::References),
            "contains" => Ok(Self::Contains),
            other => Err(UnknownRelationshipKind(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownRelationshipKind(pub String);

impl std::fmt::Display for UnknownRelationshipKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown relationship kind: {}", self.0)
    }
}

impl std::error::Error for UnknownRelationshipKind {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relationship {
    pub source: SymbolId,
    pub target: SymbolId,
    pub kind: RelationshipKind,
}

impl Relationship {
    pub fn new(source: SymbolId, target: SymbolId, kind: RelationshipKind) -> Self {
        Self {
            source,
            target,
            kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relationship_connects_two_symbols() {
        let source = SymbolId::new();
        let target = SymbolId::new();
        let rel = Relationship::new(source, target, RelationshipKind::Calls);
        assert_eq!(rel.source, source);
        assert_eq!(rel.target, target);
        assert_eq!(rel.kind, RelationshipKind::Calls);
    }
}
