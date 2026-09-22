use serde::{Deserialize, Serialize};

use crate::ids::{AnalysisRunId, RepositoryId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IndexStage {
    Queued,
    Scanning,
    Parsing,
    Resolving,
    Embedding,
    Persisting,
    Succeeded,
    Failed,
}

impl IndexStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Scanning => "scanning",
            Self::Parsing => "parsing",
            Self::Resolving => "resolving",
            Self::Embedding => "embedding",
            Self::Persisting => "persisting",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

impl std::fmt::Display for IndexStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for IndexStage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(Self::Queued),
            "scanning" => Ok(Self::Scanning),
            "parsing" => Ok(Self::Parsing),
            "resolving" => Ok(Self::Resolving),
            "embedding" => Ok(Self::Embedding),
            "persisting" => Ok(Self::Persisting),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            other => Err(format!("unknown index stage: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    pub stage: IndexStage,
    pub message: String,
    pub files_discovered: u64,
    pub files_parsed: u64,
    pub files_skipped: u64,
    pub symbols_extracted: u64,
    pub relationships_built: u64,
    pub chunks_embedded: u64,
    pub duration_ms: u64,
}

impl IndexProgress {
    pub fn queued() -> Self {
        Self {
            stage: IndexStage::Queued,
            message: "Index job queued".into(),
            files_discovered: 0,
            files_parsed: 0,
            files_skipped: 0,
            symbols_extracted: 0,
            relationships_built: 0,
            chunks_embedded: 0,
            duration_ms: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisRun {
    pub id: AnalysisRunId,
    pub repository_id: Option<RepositoryId>,
    pub source: String,
    pub status: IndexStage,
    pub progress: IndexProgress,
    pub error: Option<String>,
    pub started_at: i64,
    pub finished_at: Option<i64>,
}

impl AnalysisRun {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            id: AnalysisRunId::new(),
            repository_id: None,
            source: source.into(),
            status: IndexStage::Queued,
            progress: IndexProgress::queued(),
            error: None,
            started_at: unix_now(),
            finished_at: None,
        }
    }
}

pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
