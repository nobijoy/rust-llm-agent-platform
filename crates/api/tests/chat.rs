mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{MockModel, spawn_app, spawn_app_with_model};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

async fn post_chat(router: axum::Router, prompt: &str) -> axum::http::Response<Body> {
    let body = serde_json::json!({ "prompt": prompt }).to_string();
    router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/chat")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("router responded")
}

async fn read_json(response: axum::http::Response<Body>) -> Value {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();
    serde_json::from_slice(&bytes).expect("body is json")
}

#[tokio::test]
async fn chat_rejects_empty_prompt() {
    let app = spawn_app().await;
    let response = post_chat(app.router, "   ").await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = read_json(response).await;
    assert_eq!(body["error"], "prompt cannot be empty");
}

fn parse_tool_envelope(response_field: &Value) -> Value {
    let raw = response_field
        .as_str()
        .expect("response field must be a json string envelope");
    serde_json::from_str(raw).expect("tool envelope must be valid json")
}

#[tokio::test]
async fn chat_routes_arithmetic_to_tool_without_calling_model() {
    let mock = MockModel::with_response("MODEL_MUST_NOT_BE_CALLED");
    let app = spawn_app_with_model(mock).await;
    let model = app.model.clone();
    let storage = app.storage.clone();

    let response = post_chat(app.router, "please add 7 and 35").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    assert!(body["session_id"].as_str().is_some());

    let envelope = parse_tool_envelope(&body["response"]);
    assert_eq!(envelope["tool_name"], "add");
    assert_eq!(envelope["result"]["value"], 42);
    assert!(envelope["confidence"].as_f64().is_some());

    assert!(
        model.last_user_prompt.lock().unwrap().is_none(),
        "tool-routed prompts must not hit the model"
    );

    let persisted = storage.recent_runs(10).await.expect("read runs");
    assert_eq!(persisted.len(), 1);
    let persisted_envelope: Value =
        serde_json::from_str(&persisted[0].response).expect("persisted response is json envelope");
    assert_eq!(persisted_envelope["tool_name"], "add");
    assert_eq!(persisted_envelope["result"]["value"], 42);
}

#[tokio::test]
async fn chat_routes_calculator_to_tool() {
    let app = spawn_app().await;
    let response = post_chat(app.router, "calc 2 + 3 * 4").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    let envelope = parse_tool_envelope(&body["response"]);
    assert_eq!(envelope["tool_name"], "calculator");
    assert_eq!(envelope["result"]["expression"], "2 + 3 * 4");
    assert_eq!(envelope["result"]["value"].as_f64(), Some(14.0));
}

#[tokio::test]
async fn chat_falls_back_to_model_for_open_ended_prompts() {
    let mock = MockModel::with_response("the meaning is 42");
    let app = spawn_app_with_model(mock).await;
    let model = app.model.clone();

    let response = post_chat(app.router, "what is the meaning of life?").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    assert_eq!(body["response"], "the meaning is 42");

    let captured_prompt = model
        .last_user_prompt
        .lock()
        .unwrap()
        .clone()
        .expect("model received a prompt");
    assert_eq!(captured_prompt, "what is the meaning of life?");

    let captured_system = model.last_system_prompt.lock().unwrap().clone();
    assert!(
        captured_system.is_some(),
        "system prompt should be forwarded to the model"
    );
}

#[tokio::test]
async fn chat_rejects_malformed_json() {
    let app = spawn_app().await;

    let response = app
        .router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/chat")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{not json"))
                .unwrap(),
        )
        .await
        .expect("router responded");

    assert!(
        response.status().is_client_error(),
        "malformed json must return a 4xx, got {}",
        response.status()
    );
}
