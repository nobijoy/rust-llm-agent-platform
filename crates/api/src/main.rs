use std::net::SocketAddr;
use std::sync::Arc;

use agent::AgentService;
use api::{AppState, build_router};
use common::{AppConfig, init_tracing};
use model::OllamaGateway;
use storage::Storage;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let config = AppConfig::from_env()?;
    info!("starting api server");

    let storage = Storage::connect(&config.database_url).await?;
    let model = Arc::new(OllamaGateway::new(
        config.ollama_url.clone(),
        config.ollama_model.clone(),
    ));
    let state = AppState::new(storage, AgentService::new(), model);

    let app = build_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("server listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
