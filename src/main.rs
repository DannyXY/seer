mod api;
mod config;
mod db;
mod errors;
mod jobs;
mod models;
mod services;
mod telemetry;

use std::net::SocketAddr;
use std::sync::Arc;

use api::router;
use config::{AppRole, Settings};
use services::AppServices;
use tokio::net::TcpListener;
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub settings: Settings,
    pub services: Arc<AppServices>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    telemetry::init();

    let settings = Settings::from_env()?;
    let services = Arc::new(AppServices::new(settings.clone()).await?);
    let state = AppState {
        settings: settings.clone(),
        services,
    };

    match settings.app_role {
        AppRole::Api => run_api(state).await,
        AppRole::Worker => jobs::run_worker(state).await,
    }
}

async fn run_api(state: AppState) -> anyhow::Result<()> {
    jobs::spawn_internal_jobs(state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], state.settings.port));
    let listener = TcpListener::bind(addr).await?;
    info!(%addr, "seer api listening");

    axum::serve(listener, router(state)).await?;
    Ok(())
}
