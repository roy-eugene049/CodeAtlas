use std::path::PathBuf;
use std::sync::Arc;

use codeatlas_storage::Store;

use crate::jobs::JobHub;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    pub cache_dir: PathBuf,
    pub jobs: Arc<JobHub>,
}
