use std::env;

use anyhow::{Context, Result};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub ollama_url: String,
    pub ollama_model: String,
    pub database_url: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            host: env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("APP_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("APP_PORT must be a valid u16")?,
            ollama_url: env::var("OLLAMA_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string()),
            ollama_model: env::var("OLLAMA_MODEL")
                .unwrap_or_else(|_| "llama3.1:8b-instruct-q4_K_M".to_string()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://./data/agent.db".to_string()),
        })
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    External(String),
    #[error("{0}")]
    Internal(String),
}

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub error: String,
}

pub fn init_tracing() {
    let filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "info,tower_http=info,api=debug,agent=debug".to_string());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .json()
        .init();
}
