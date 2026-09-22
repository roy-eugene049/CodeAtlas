use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use codeatlas_domain::{unix_now, AnalysisRun, AnalysisRunId, IndexProgress, IndexStage, RepositoryId};
use serde::Serialize;
use tokio::sync::broadcast;

use codeatlas_storage::Store;

const EVENT_CAPACITY: usize = 64;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexEvent {
    pub job_id: AnalysisRunId,
    pub repository_id: Option<RepositoryId>,
    #[serde(flatten)]
    pub progress: IndexProgress,
    pub error: Option<String>,
}

impl From<&AnalysisRun> for IndexEvent {
    fn from(run: &AnalysisRun) -> Self {
        Self {
            job_id: run.id,
            repository_id: run.repository_id,
            progress: run.progress.clone(),
            error: run.error.clone(),
        }
    }
}

struct JobHandle {
    run: AnalysisRun,
    tx: broadcast::Sender<IndexEvent>,
}

pub struct JobHub {
    jobs: Mutex<HashMap<AnalysisRunId, JobHandle>>,
}

impl JobHub {
    pub fn new() -> Self {
        Self {
            jobs: Mutex::new(HashMap::new()),
        }
    }

    pub fn enqueue(&self, source: impl Into<String>) -> AnalysisRun {
        let run = AnalysisRun::new(source);
        self.insert(run.clone());
        run
    }

    pub fn insert(&self, run: AnalysisRun) {
        let (tx, _) = broadcast::channel(EVENT_CAPACITY);
        let mut jobs = self.jobs.lock().unwrap_or_else(|error| error.into_inner());
        jobs.insert(run.id, JobHandle { run, tx });
    }

    pub fn get(&self, id: AnalysisRunId) -> Option<AnalysisRun> {
        let jobs = self.jobs.lock().unwrap_or_else(|error| error.into_inner());
        jobs.get(&id).map(|handle| handle.run.clone())
    }

    pub fn running_for_source(&self, source: &str) -> Option<AnalysisRun> {
        let jobs = self.jobs.lock().unwrap_or_else(|error| error.into_inner());
        jobs.values()
            .map(|handle| &handle.run)
            .find(|run| {
                run.source == source
                    && !matches!(run.status, IndexStage::Succeeded | IndexStage::Failed)
            })
            .cloned()
    }

    pub fn subscribe(&self, id: AnalysisRunId) -> Option<(AnalysisRun, broadcast::Receiver<IndexEvent>)> {
        let jobs = self.jobs.lock().unwrap_or_else(|error| error.into_inner());
        let handle = jobs.get(&id)?;
        Some((handle.run.clone(), handle.tx.subscribe()))
    }

    pub fn publish(&self, store: &Store, mut run: AnalysisRun) {
        if matches!(run.progress.stage, IndexStage::Succeeded | IndexStage::Failed) {
            run.status = run.progress.stage;
            run.finished_at = Some(unix_now());
        } else {
            run.status = run.progress.stage;
        }
        let _ = store.save_run(&run);
        let mut jobs = self.jobs.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(handle) = jobs.get_mut(&run.id) {
            handle.run = run.clone();
            let _ = handle.tx.send(IndexEvent::from(&run));
        }
    }
}

impl Default for JobHub {
    fn default() -> Self {
        Self::new()
    }
}

pub fn spawn_index_job(state: crate::state::AppState, job_id: AnalysisRunId) {
    tokio::spawn(async move {
        let store = state.store.clone();
        let cache_dir = state.cache_dir.clone();
        let jobs = state.jobs.clone();
        let Some(run) = jobs.get(job_id) else {
            return;
        };
        let source = run.source.clone();
        let jobs_for_progress = jobs.clone();
        let store_for_progress = store.clone();
        let result = tokio::task::spawn_blocking(move || {
            let service = crate::service::RepositoryService::new(store, cache_dir);
            service.run_index(&source, |progress| {
                if let Some(mut current) = jobs_for_progress.get(job_id) {
                    current.progress = progress;
                    jobs_for_progress.publish(&store_for_progress, current);
                }
            })
        })
        .await;

        match result {
            Ok(Ok((overview, repository_id))) => {
                if let Some(mut current) = jobs.get(job_id) {
                    current.repository_id = Some(repository_id);
                    current.progress.stage = IndexStage::Succeeded;
                    current.progress.message = format!(
                        "Indexed {} symbols in {}",
                        overview.stats.symbol_count, overview.repository.name
                    );
                    current.error = None;
                    jobs.publish(&state.store, current);
                }
            }
            Ok(Err(error)) => fail(&jobs, &state.store, job_id, error.to_string()),
            Err(error) => fail(&jobs, &state.store, job_id, error.to_string()),
        }
    });
}

fn fail(jobs: &Arc<JobHub>, store: &Store, job_id: AnalysisRunId, message: String) {
    if let Some(mut current) = jobs.get(job_id) {
        current.progress.stage = IndexStage::Failed;
        current.progress.message = message.clone();
        current.error = Some(message);
        jobs.publish(store, current);
    }
}
