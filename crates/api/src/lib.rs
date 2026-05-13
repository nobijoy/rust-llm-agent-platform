use std::sync::Arc;

use agent::{AgentService, MemoryRecord, ToolContext};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Json, Router, routing::get, routing::post};
use common::ApiErrorBody;
use model::ModelGateway;
use serde::{Deserialize, Serialize};
use storage::Storage;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub storage: Storage,
    pub agent: AgentService,
    pub model: Arc<dyn ModelGateway>,
}

impl AppState {
    pub fn new(storage: Storage, agent: AgentService, model: Arc<dyn ModelGateway>) -> Self {
        Self {
            storage,
            agent,
            model,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub session_id: String,
    pub response: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/chat", post(chat))
        .route("/api/v1/runs/{limit}", get(recent_runs))
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            std::time::Duration::from_secs(120),
        ))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
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
pub enum ApiError {
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
