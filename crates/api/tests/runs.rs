mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::spawn_app;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

async fn get_runs(router: axum::Router, limit: i64) -> axum::http::Response<Body> {
    router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/runs/{limit}"))
                .body(Body::empty())
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
async fn recent_runs_returns_empty_list_initially() {
    let app = spawn_app().await;
    let response = get_runs(app.router, 10).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    let arr = body.as_array().expect("body is array");
    assert!(arr.is_empty());
}

#[tokio::test]
async fn recent_runs_returns_saved_rows_in_descending_order() {
    let app = spawn_app().await;

    app.storage
        .save_run("session-1", "first prompt", "first response")
        .await
        .expect("save run 1");
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    app.storage
        .save_run("session-2", "second prompt", "second response")
        .await
        .expect("save run 2");

    let response = get_runs(app.router, 10).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    let arr = body.as_array().expect("body is array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["user_prompt"], "second prompt");
    assert_eq!(arr[1]["user_prompt"], "first prompt");
}

#[tokio::test]
async fn recent_runs_clamps_limit_above_max() {
    let app = spawn_app().await;
    for i in 0..3 {
        app.storage
            .save_run(
                &format!("session-{i}"),
                &format!("prompt-{i}"),
                &format!("response-{i}"),
            )
            .await
            .expect("save");
    }

    let response = get_runs(app.router, 9_999).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    let arr = body.as_array().expect("body is array");
    assert_eq!(arr.len(), 3, "limit must be clamped, not error");
}

#[tokio::test]
async fn recent_runs_clamps_zero_to_minimum_one() {
    let app = spawn_app().await;
    app.storage
        .save_run("session-only", "only prompt", "only response")
        .await
        .expect("save");

    let response = get_runs(app.router, 0).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = read_json(response).await;
    let arr = body.as_array().expect("body is array");
    assert_eq!(arr.len(), 1, "zero limit must be clamped to 1");
}

#[tokio::test]
async fn recent_runs_rejects_non_integer_limit() {
    let app = spawn_app().await;
    let response = app
        .router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/runs/not-a-number")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("router responded");

    assert!(
        response.status().is_client_error(),
        "non-integer path param must be 4xx, got {}",
        response.status()
    );
}
