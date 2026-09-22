use codeatlas_domain::{
    FileId, GitEvolution, IndexStats, Language, Relationship, Repository, RepositoryId, SourceFile,
    Symbol, SymbolId, SymbolKind,
};
use codeatlas_ai::{
    AnswerMode, Citation, GroundedAnswer, ImpactNarrative, QueryIntent, SymbolExplanation,
};
use codeatlas_graph::{
    ArchitectureLayer, ArchitectureOutline, GraphView, ImpactReport, RepoHealth, SymbolRef,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexRepositoryRequest {
    pub source: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexJobResponse {
    pub job_id: codeatlas_domain::AnalysisRunId,
    pub repository_id: Option<RepositoryId>,
    pub status: codeatlas_domain::IndexStage,
    pub progress: codeatlas_domain::IndexProgress,
    pub error: Option<String>,
}

impl From<&codeatlas_domain::AnalysisRun> for IndexJobResponse {
    fn from(run: &codeatlas_domain::AnalysisRun) -> Self {
        Self {
            job_id: run.id,
            repository_id: run.repository_id,
            status: run.status,
            progress: run.progress.clone(),
            error: run.error.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositorySummary {
    pub id: RepositoryId,
    pub name: String,
    pub url: String,
    pub default_branch: String,
    pub commit_sha: String,
    pub line_count: u64,
}

impl RepositorySummary {
    pub fn from_repo(repo: Repository, line_count: u64) -> Self {
        Self {
            id: repo.id,
            name: repo.name,
            url: repo.url,
            default_branch: repo.default_branch,
            commit_sha: repo.commit_sha,
            line_count,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewResponse {
    pub repository: RepositorySummary,
    pub stats: IndexStats,
    pub architecture: Vec<ArchitectureLayer>,
    pub architecture_tree: ArchitectureOutline,
    pub health: RepoHealth,
    pub languages: Vec<LanguageCount>,
    pub ai_summary: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageCount {
    pub language: Language,
    pub file_count: u64,
    pub percent: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContents {
    pub path: String,
    pub language: Language,
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileResponse {
    pub id: FileId,
    pub path: String,
    pub language: Language,
    pub size_bytes: u64,
}

impl From<SourceFile> for FileResponse {
    fn from(file: SourceFile) -> Self {
        Self {
            id: file.id,
            path: file.path,
            language: file.language,
            size_bytes: file.size_bytes,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolResponse {
    pub id: SymbolId,
    pub file_id: FileId,
    pub name: String,
    pub kind: SymbolKind,
    pub start_line: u32,
    pub end_line: u32,
    pub path: String,
}

impl SymbolResponse {
    pub fn from_symbol(symbol: Symbol, path: impl Into<String>) -> Self {
        Self {
            id: symbol.id,
            file_id: symbol.file_id,
            name: symbol.name,
            kind: symbol.kind,
            start_line: symbol.start_line,
            end_line: symbol.end_line,
            path: path.into(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphResponse {
    #[serde(flatten)]
    pub view: GraphView,
    pub focus: Option<SymbolId>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpactResponse {
    #[serde(flatten)]
    pub report: ImpactReport,
    pub narrative: ImpactNarrative,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplainResponse {
    pub symbol: SymbolResponse,
    pub purpose: String,
    pub flow: Vec<String>,
    pub dependencies: Vec<SymbolRef>,
    pub dependents: Vec<SymbolRef>,
    pub citations: Vec<Citation>,
    pub mode: AnswerMode,
    pub text: String,
}

impl ExplainResponse {
    pub fn from_parts(symbol: SymbolResponse, explanation: SymbolExplanation) -> Self {
        Self {
            symbol,
            purpose: explanation.purpose,
            flow: explanation.flow,
            dependencies: explanation.dependencies,
            dependents: explanation.dependents,
            citations: explanation.citations,
            mode: explanation.mode,
            text: explanation.text,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipResponse {
    pub source: SymbolId,
    pub target: SymbolId,
    pub kind: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskRequest {
    pub question: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AskResponse {
    pub question: String,
    pub intent: QueryIntent,
    pub mode: AnswerMode,
    pub answer: String,
    pub citations: Vec<Citation>,
    pub confidence: codeatlas_ai::ContextConfidence,
    pub retrieval_summary: String,
}

impl AskResponse {
    pub fn from_answer(question: String, answer: GroundedAnswer) -> Self {
        Self {
            question,
            intent: answer.intent,
            mode: answer.mode,
            answer: answer.text,
            citations: answer.citations,
            confidence: answer.confidence,
            retrieval_summary: answer.retrieval_summary,
        }
    }
}

pub type EvolutionResponse = GitEvolution;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub id: SymbolId,
    pub name: String,
    pub path: String,
    pub kind: String,
    pub score: f32,
    pub source: String,
}

impl From<codeatlas_search::SearchResult> for SearchHit {
    fn from(hit: codeatlas_search::SearchResult) -> Self {
        Self {
            id: hit.symbol_id,
            name: hit.name,
            path: hit.path,
            kind: hit.kind,
            score: hit.score,
            source: match hit.source {
                codeatlas_search::SearchSource::SymbolName => "symbolName".into(),
                codeatlas_search::SearchSource::FileName => "fileName".into(),
                codeatlas_search::SearchSource::Path => "path".into(),
                codeatlas_search::SearchSource::Text => "text".into(),
                codeatlas_search::SearchSource::Semantic => "semantic".into(),
            },
        }
    }
}

impl From<Relationship> for RelationshipResponse {
    fn from(rel: Relationship) -> Self {
        Self {
            source: rel.source,
            target: rel.target,
            kind: rel.kind.as_str().to_string(),
        }
    }
}
