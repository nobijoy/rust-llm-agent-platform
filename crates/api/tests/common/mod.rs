use std::sync::Arc;
use std::sync::Mutex;

use agent::AgentService;
use api::{AppState, build_router};
use async_trait::async_trait;
use axum::Router;
use model::{ModelError, ModelGateway};
use storage::Storage;
use tempfile::TempDir;

#[derive(Debug, Default, Clone)]
pub struct MockModel {
    pub canned_response: String,
    pub last_user_prompt: Arc<Mutex<Option<String>>>,
    pub last_system_prompt: Arc<Mutex<Option<String>>>,
}

impl MockModel {
    pub fn with_response(canned: impl Into<String>) -> Self {
        Self {
            canned_response: canned.into(),
            ..Default::default()
        }
    }
}

#[async_trait]
impl ModelGateway for MockModel {
    async fn prompt(&self, user_prompt: &str, system_prompt: &str) -> Result<String, ModelError> {
        *self.last_user_prompt.lock().expect("lock poisoned") = Some(user_prompt.to_string());
        *self.last_system_prompt.lock().expect("lock poisoned") = Some(system_prompt.to_string());
        Ok(self.canned_response.clone())
    }
}

#[allow(dead_code)]
pub struct TestApp {
    pub router: Router,
    pub storage: Storage,
    pub model: Arc<MockModel>,
    _tempdir: TempDir,
}

pub async fn spawn_app_with_model(model: MockModel) -> TestApp {
    let tempdir = tempfile::tempdir().expect("create tempdir");
    let db_path = tempdir.path().join("agent.db");
    let database_url = format!("sqlite://{}", db_path.to_string_lossy());

    let storage = Storage::connect(&database_url)
        .await
        .expect("connect storage to test db");

    let model = Arc::new(model);
    let model_dyn: Arc<dyn ModelGateway> = model.clone();
    let state = AppState::new(storage.clone(), AgentService::new(), model_dyn);
    let router = build_router(state);

    TestApp {
        router,
        storage,
        model,
        _tempdir: tempdir,
    }
}

pub async fn spawn_app() -> TestApp {
    spawn_app_with_model(MockModel::with_response("mock-model-response")).await
}
