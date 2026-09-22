use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn from_uuid(id: Uuid) -> Self {
                Self(id)
            }

            pub fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(Uuid::parse_str(s)?))
            }
        }
    };
}

id_newtype!(RepositoryId);
id_newtype!(FileId);
id_newtype!(SymbolId);
id_newtype!(AnalysisRunId);

impl FileId {
    pub fn stable(repository_id: RepositoryId, path: &str) -> Self {
        Self(Uuid::new_v5(&repository_id.as_uuid(), path.as_bytes()))
    }
}

impl SymbolId {
    pub fn stable(
        file_id: FileId,
        name: &str,
        kind: crate::symbol::SymbolKind,
        start_line: u32,
    ) -> Self {
        let key = format!("{}:{}:{start_line}", name, kind.as_str());
        Self(Uuid::new_v5(&file_id.as_uuid(), key.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        assert_ne!(RepositoryId::new(), RepositoryId::new());
        assert_ne!(FileId::new(), FileId::new());
        assert_ne!(SymbolId::new(), SymbolId::new());
    }

    #[test]
    fn ids_round_trip_as_strings() {
        let id = SymbolId::new();
        let parsed: SymbolId = id.to_string().parse().expect("parse id");
        assert_eq!(id, parsed);
    }

    #[test]
    fn stable_ids_follow_path_and_symbol_identity() {
        let repo = RepositoryId::new();
        let left = FileId::stable(repo, "src/auth.ts");
        let right = FileId::stable(repo, "src/auth.ts");
        assert_eq!(left, right);
        assert_ne!(left, FileId::stable(repo, "src/other.ts"));
        let kind = crate::symbol::SymbolKind::Function;
        assert_eq!(
            SymbolId::stable(left, "login", kind, 10),
            SymbolId::stable(left, "login", kind, 10)
        );
    }
}
