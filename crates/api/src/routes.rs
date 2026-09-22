use std::convert::Infallible;
use std::str::FromStr;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use axum::{Json, Router};
use codeatlas_domain::{AnalysisRunId, RepositoryId, SymbolId, SymbolKind};
use futures::StreamExt;
use serde::Deserialize;
use tokio_stream::wrappers::BroadcastStream;

use crate::dto::{
    AskRequest, AskResponse, EvolutionResponse, ExplainResponse, FileContents, FileResponse,
    GraphResponse, ImpactResponse, IndexJobResponse, IndexRepositoryRequest, OverviewResponse,
    RepositorySummary, SearchHit, SymbolResponse,
};
use crate::error::ApiError;
use crate::jobs::{spawn_index_job, IndexEvent};
use crate::service::RepositoryService;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/repositories", get(list_repositories).post(create_repository))
        .route("/api/repositories/{id}", get(get_overview))
        .route("/api/repositories/{id}/overview", get(get_overview))
        .route("/api/repositories/{id}/index", axum::routing::post(reindex_repository))
        .route("/api/repositories/{id}/files", get(get_files))
        .route("/api/repositories/{id}/contents", get(get_contents))
        .route("/api/repositories/{id}/symbols", get(get_symbols))
        .route("/api/repositories/{id}/graph", get(get_graph))
        .route("/api/repositories/{id}/search", get(search_symbols))
        .route(
            "/api/repositories/{id}/symbols/{symbol_id}/impact",
            get(get_impact),
        )
        .route(
            "/api/repositories/{id}/symbols/{symbol_id}/explain",
            get(get_explain),
        )
        .route(
            "/api/repositories/{id}/symbols/{symbol_id}/dependencies",
            get(get_dependencies),
        )
        .route(
            "/api/repositories/{id}/symbols/{symbol_id}/dependents",
            get(get_dependents),
        )
        .route(
            "/api/repositories/{id}/symbols/{symbol_id}/related",
            get(get_related),
        )
        .route("/api/repositories/{id}/ask", axum::routing::post(ask))
        .route("/api/repositories/{id}/evolution", get(get_evolution))
        .route("/api/symbols/{symbol_id}", get(get_symbol))
        .route("/api/symbols/{symbol_id}/impact", get(get_symbol_impact))
        .route("/api/symbols/{symbol_id}/explain", get(get_symbol_explain))
        .route("/api/jobs/{id}", get(get_job))
        .route("/api/jobs/{id}/events", get(job_events))
        .with_state(state)
}

async fn create_repository(
    State(state): State<AppState>,
    Json(body): Json<IndexRepositoryRequest>,
) -> Result<(StatusCode, Json<IndexJobResponse>), ApiError> {
    enqueue_index(state, body.source, None)
}

async fn reindex_repository(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<IndexJobResponse>), ApiError> {
    let repo_id = parse_id(&id)?;
    let overview = service(&state).overview(repo_id)?;
    enqueue_index(state, overview.repository.url, Some(repo_id))
}

fn enqueue_index(
    state: AppState,
    source: String,
    repository_id: Option<RepositoryId>,
) -> Result<(StatusCode, Json<IndexJobResponse>), ApiError> {
    if let Some(existing) = state.jobs.running_for_source(&source) {
        return Ok((StatusCode::ACCEPTED, Json(IndexJobResponse::from(&existing))));
    }
    let mut run = state.jobs.enqueue(source);
    run.repository_id = repository_id;
    state.jobs.publish(&state.store, run.clone());
    spawn_index_job(state.clone(), run.id);
    Ok((StatusCode::ACCEPTED, Json(IndexJobResponse::from(&run))))
}

async fn list_repositories(
    State(state): State<AppState>,
    ) -> Result<Json<Vec<RepositorySummary>>, ApiError> {
    Ok(Json(service(&state).list()?))
}

async fn get_overview(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<OverviewResponse>, ApiError> {
    Ok(Json(service(&state).overview(parse_id(&id)?)?))
}

async fn get_files(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<FileResponse>>, ApiError> {
    Ok(Json(service(&state).files(parse_id(&id)?)?))
}

#[derive(Debug, Deserialize)]
struct ContentsQuery {
    path: String,
}

async fn get_contents(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ContentsQuery>,
) -> Result<Json<FileContents>, ApiError> {
    Ok(Json(service(&state).file_contents(parse_id(&id)?, &query.path)?))
}

#[derive(Debug, Deserialize)]
struct SymbolQuery {
    q: Option<String>,
    kind: Option<String>,
}

async fn get_symbols(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<SymbolQuery>,
) -> Result<Json<Vec<SymbolResponse>>, ApiError> {
    let kind = query
        .kind
        .as_deref()
        .map(SymbolKind::from_str)
        .transpose()
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    Ok(Json(service(&state).symbols(parse_id(&id)?, query.q.as_deref(), kind)?))
}

#[derive(Debug, Deserialize)]
struct GraphQuery {
    focus: Option<String>,
    depth: Option<u8>,
}

async fn get_graph(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<GraphQuery>,
) -> Result<Json<GraphResponse>, ApiError> {
    let focus = query
        .focus
        .as_deref()
        .map(parse_symbol_id)
        .transpose()?;
    Ok(Json(service(&state).graph(
        parse_id(&id)?,
        focus,
        query.depth.unwrap_or(2),
    )?))
}

async fn get_dependencies(
    State(state): State<AppState>,
    Path((id, symbol_id)): Path<(String, String)>,
) -> Result<Json<Vec<codeatlas_graph::SymbolRef>>, ApiError> {
    Ok(Json(
        service(&state).dependencies(parse_id(&id)?, parse_symbol_id(&symbol_id)?)?,
    ))
}

async fn get_dependents(
    State(state): State<AppState>,
    Path((id, symbol_id)): Path<(String, String)>,
) -> Result<Json<Vec<codeatlas_graph::SymbolRef>>, ApiError> {
    Ok(Json(
        service(&state).dependents(parse_id(&id)?, parse_symbol_id(&symbol_id)?)?,
    ))
}

async fn get_related(
    State(state): State<AppState>,
    Path((id, symbol_id)): Path<(String, String)>,
) -> Result<Json<Vec<codeatlas_graph::SymbolRef>>, ApiError> {
    Ok(Json(
        service(&state).related(parse_id(&id)?, parse_symbol_id(&symbol_id)?)?,
    ))
}

async fn get_explain(
    State(state): State<AppState>,
    Path((id, symbol_id)): Path<(String, String)>,
) -> Result<Json<ExplainResponse>, ApiError> {
    let service = service(&state);
    let repo_id = parse_id(&id)?;
    let symbol = parse_symbol_id(&symbol_id)?;
    let explanation = tokio::task::spawn_blocking(move || service.explain(repo_id, symbol))
        .await
        .map_err(|error| ApiError::internal(error.to_string()))??;
    Ok(Json(explanation))
}

async fn get_impact(
    State(state): State<AppState>,
    Path((id, symbol_id)): Path<(String, String)>,
) -> Result<Json<ImpactResponse>, ApiError> {
    Ok(Json(
        service(&state).impact(parse_id(&id)?, parse_symbol_id(&symbol_id)?)?,
    ))
}

async fn get_symbol(
    State(state): State<AppState>,
    Path(symbol_id): Path<String>,
) -> Result<Json<SymbolResponse>, ApiError> {
    Ok(Json(service(&state).symbol(parse_symbol_id(&symbol_id)?)?))
}

async fn get_symbol_impact(
    State(state): State<AppState>,
    Path(symbol_id): Path<String>,
) -> Result<Json<ImpactResponse>, ApiError> {
    Ok(Json(
        service(&state).impact_by_symbol(parse_symbol_id(&symbol_id)?)?,
    ))
}

async fn get_symbol_explain(
    State(state): State<AppState>,
    Path(symbol_id): Path<String>,
) -> Result<Json<ExplainResponse>, ApiError> {
    let service = service(&state);
    let symbol = parse_symbol_id(&symbol_id)?;
    let explanation = tokio::task::spawn_blocking(move || service.explain_by_symbol(symbol))
        .await
        .map_err(|error| ApiError::internal(error.to_string()))??;
    Ok(Json(explanation))
}

async fn get_evolution(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<EvolutionResponse>, ApiError> {
    Ok(Json(service(&state).evolution(parse_id(&id)?)?))
}

async fn ask(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AskRequest>,
) -> Result<Json<AskResponse>, ApiError> {
    let service = service(&state);
    let repo_id = parse_id(&id)?;
    let question = body.question;
    let answer = tokio::task::spawn_blocking(move || service.ask(repo_id, &question))
        .await
        .map_err(|error| ApiError::internal(error.to_string()))??;
    Ok(Json(answer))
}

async fn search_symbols(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<SymbolQuery>,
) -> Result<Json<Vec<SearchHit>>, ApiError> {
    let service = service(&state);
    let repo_id = parse_id(&id)?;
    let q = query.q.unwrap_or_default();
    let hits = tokio::task::spawn_blocking(move || service.search(repo_id, &q))
        .await
        .map_err(|error| ApiError::internal(error.to_string()))??;
    Ok(Json(hits))
}

async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<IndexJobResponse>, ApiError> {
    let job_id = parse_job_id(&id)?;
    let run = state
        .jobs
        .get(job_id)
        .or_else(|| state.store.load_run(job_id).ok().flatten())
        .ok_or_else(|| ApiError::not_found("index job not found"))?;
    Ok(Json(IndexJobResponse::from(&run)))
}

async fn job_events(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let job_id = parse_job_id(&id)?;
    let (run, rx) = state
        .jobs
        .subscribe(job_id)
        .ok_or_else(|| ApiError::not_found("index job not found"))?;
    let initial = IndexEvent::from(&run);
    let head = futures::stream::once(async move {
        Ok(Event::default()
            .json_data(initial)
            .unwrap_or_else(|_| Event::default().data("{}")))
    });
    let tail = BroadcastStream::new(rx).filter_map(|item| async move {
        let event = item.ok()?;
        Some(Ok(Event::default()
            .json_data(event)
            .unwrap_or_else(|_| Event::default().data("{}"))))
    });
    Ok(Sse::new(head.chain(tail)).keep_alive(KeepAlive::default()))
}

fn service(state: &AppState) -> RepositoryService {
    RepositoryService::new(state.store.clone(), state.cache_dir.clone())
}

fn parse_id(id: &str) -> Result<RepositoryId, ApiError> {
    id.parse()
        .map_err(|_| ApiError::bad_request("invalid repository id"))
}

fn parse_symbol_id(id: &str) -> Result<SymbolId, ApiError> {
    id.parse()
        .map_err(|_| ApiError::bad_request("invalid symbol id"))
}

fn parse_job_id(id: &str) -> Result<AnalysisRunId, ApiError> {
    id.parse()
        .map_err(|_| ApiError::bad_request("invalid job id"))
}
