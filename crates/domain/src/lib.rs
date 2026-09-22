//! Core representation of a codebase: repositories, files, symbols, and relationships.
//!
//! This crate has no HTTP, database, parser, or LLM dependencies.

mod evolution;
mod file;
mod ids;
mod index;
mod language;
mod relationship;
mod repository;
mod run;
mod sensitive;
mod symbol;
mod unit;

pub use evolution::{
    AuthorSummary, BranchSummary, CommitSummary, CommitTouch, FileHotspot, GitEvolution,
};
pub use file::SourceFile;
pub use ids::{AnalysisRunId, FileId, RepositoryId, SymbolId};
pub use index::{IndexStats, RepositoryIndex};
pub use language::{Language, UnknownLanguage};
pub use relationship::{Relationship, RelationshipKind, UnknownRelationshipKind};
pub use repository::Repository;
pub use run::{unix_now, AnalysisRun, IndexProgress, IndexStage};
pub use sensitive::{is_sensitive_path, looks_like_secret, redact_secrets};
pub use symbol::{Symbol, SymbolKind, UnknownSymbolKind};
pub use unit::SemanticUnit;
