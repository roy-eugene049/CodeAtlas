//! File scan → ignore → language detect → parse → symbols/imports → graph → chunks.
//!
//! Unchanged files skip tree-sitter. Prefer a git commit range
//! (added / modified / deleted / renamed) plus a dirty working-tree check.
//! Content hashes still catch anything git did not report. Tree-sitter's
//! edit-based incremental parse applies when we have a byte-range edit.
//!
//! Parse work is CPU-bound, so it uses a bounded rayon pool rather than
//! unbounded `tokio::spawn` per file.

mod error;
mod limits;
mod pipeline;
mod resolve;
mod source;
mod units;
mod walk;

pub use error::IndexError;
pub use limits::IndexLimits;
pub use pipeline::{IndexOutcome, Indexer};
pub use source::{materialize, materialize_git, materialize_local, MaterializedSource};
pub use walk::{discover_files, discover_files_with_limits};
