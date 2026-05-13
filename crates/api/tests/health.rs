mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::spawn_app;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_ok_status() {
    let app = spawn_app().await;

    let response = app
        .router
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router responded");

    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response
        .into_body()
        .collect()
        .await
        .expect("read body")
        .to_bytes();
    let body: Value = serde_json::from_slice(&body_bytes).expect("body is json");
    assert_eq!(body["status"], "ok");
}
