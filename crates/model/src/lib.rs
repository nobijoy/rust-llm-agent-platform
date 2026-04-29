use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct OllamaGateway {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaGateway {
    pub fn new(base_url: String, model: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            model,
        }
    }
}

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("model request failed: {0}")]
    Request(String),
    #[error("model response was malformed: {0}")]
    MalformedResponse(String),
}

#[async_trait]
pub trait ModelGateway: Send + Sync {
    async fn prompt(&self, user_prompt: &str, system_prompt: &str) -> Result<String, ModelError>;
}

#[derive(Debug, Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

#[async_trait]
impl ModelGateway for OllamaGateway {
    async fn prompt(&self, user_prompt: &str, system_prompt: &str) -> Result<String, ModelError> {
        let combined = format!("{system_prompt}\n\nUser: {user_prompt}\nAssistant:");
        let payload = OllamaRequest {
            model: &self.model,
            prompt: combined,
            stream: false,
        };
        debug!("issuing ollama request");
        let response = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| ModelError::Request(e.to_string()))?;

        if !response.status().is_success() {
            return Err(ModelError::Request(format!(
                "ollama returned status {}",
                response.status()
            )));
        }

        let parsed = response
            .json::<OllamaResponse>()
            .await
            .map_err(|e| ModelError::MalformedResponse(e.to_string()))?;
        Ok(parsed.response)
    }
}
