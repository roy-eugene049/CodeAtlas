use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use codeatlas_storage::Store;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::EnvFilter;

use codeatlas_api::{jobs::JobHub, routes, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("codeatlas_api=info".parse()?))
        .init();

    let data_dir = data_dir();
    std::fs::create_dir_all(&data_dir).context("create data directory")?;
    let store = Store::open(&data_dir.join("codeatlas.db")).context("open index database")?;
    let state = AppState {
        store: Arc::new(store),
        cache_dir: data_dir.join("repos"),
        jobs: Arc::new(JobHub::new()),
    };

    let mut app = routes::router(state);
    if let Some(dashboard) = dashboard_dir() {
        let index = dashboard.join("index.html");
        app = app.fallback_service(ServeDir::new(dashboard).not_found_service(ServeFile::new(index)));
    }
    let app = app.layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    );

    let addr = listen_addr();
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind {addr}"))?;
    tracing::info!("CodeAtlas listening on http://{addr}");
    axum::serve(listener, app).await.context("serve")?;
    Ok(())
}

fn data_dir() -> PathBuf {
    std::env::var("CODEATLAS_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".data"))
}

fn listen_addr() -> SocketAddr {
    std::env::var("CODEATLAS_BIND")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 8080)))
}

fn dashboard_dir() -> Option<PathBuf> {
    let path = std::env::var("CODEATLAS_DASHBOARD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("dashboard/dist"));
    path.exists().then_some(path)
}
