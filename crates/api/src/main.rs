use std::net::SocketAddr;
use std::sync::Arc;

use agent::{AgentService, MemoryRecord, ToolContext};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Json, Router, routing::get, routing::post};
use common::{ApiErrorBody, AppConfig, init_tracing};
use model::{ModelGateway, OllamaGateway};
use serde::{Deserialize, Serialize};
use storage::Storage;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    storage: Storage,
    agent: AgentService,
    model: Arc<dyn ModelGateway>,
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    prompt: String,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    session_id: String,
    response: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

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
    let state = AppState {
        storage,
        agent: AgentService::new(),
        model,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/chat", post(chat))
        .route("/api/v1/runs/{limit}", get(recent_runs))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            std::time::Duration::from_secs(120),
        ))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("server listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(HealthResponse { status: "ok" })
}

async fn recent_runs(
    Path(limit): Path<i64>,
    State(state): State<AppState>,
) -> Result<Json<Vec<storage::AgentRun>>, ApiError> {
    let runs = state
        .storage
        .recent_runs(limit.clamp(1, 100))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(runs))
}

async fn chat(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, ApiError> {
    if payload.prompt.trim().is_empty() {
        return Err(ApiError::Validation("prompt cannot be empty".to_string()));
    }
    let session_id = Uuid::new_v4().to_string();
    debug!("chat request received");

    let response = if let Some(tool_response) = state
        .agent
        .run_tools_if_needed(
            &payload.prompt,
            &ToolContext {
                memory: state
                    .storage
                    .recent_runs(5)
                    .await
                    .map_err(|e| ApiError::Internal(e.to_string()))?
                    .into_iter()
                    .map(|run| MemoryRecord {
                        created_at: run.created_at.to_rfc3339(),
                        user_prompt: run.user_prompt,
                        response: run.response,
                    })
                    .collect(),
            },
        )
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
    {
        tool_response
    } else {
        state
            .model
            .prompt(&payload.prompt, state.agent.system_prompt())
            .await
            .map_err(|e| ApiError::External(e.to_string()))?
    };

    state
        .storage
        .save_run(&session_id, &payload.prompt, &response)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(ChatResponse {
        session_id,
        response,
    }))
}

#[derive(Debug)]
enum ApiError {
    Validation(String),
    External(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ApiError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::External(msg) => (StatusCode::BAD_GATEWAY, msg),
            ApiError::Internal(msg) => {
                error!("internal error: {msg}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };
        (status, Json(ApiErrorBody { error: message })).into_response()
    }
}
