//! Shared helpers for integration tests.
//!
//! Every DB-backed test is annotated with `#[sqlx::test]`, which gives it its
//! own freshly-migrated database and drops it when the test ends. The suite
//! therefore never touches the development database and never accumulates rows
//! across runs, so tests can assert on global state instead of being
//! pollution-tolerant.
//!
//! A reachable `DATABASE_URL` whose role may create databases is required; a
//! missing database fails loudly rather than silently skipping (skipped is not
//! passed). CI provides a throwaway Postgres service for exactly this.

#![allow(dead_code)]

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use live_chat_server::config::Config;
use live_chat_server::{AppState, build_router};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

/// A running application backed by a real database.
pub struct TestApp {
    pub router: Router,
    pub pool: PgPool,
}

/// Build the app against an isolated, already-migrated pool supplied by
/// `#[sqlx::test]`.
pub fn test_app(pool: PgPool) -> TestApp {
    let mut config = Config::from_env();
    config.jwt_secret = "integration-test-secret".to_string();
    config.max_rooms_per_user = MAX_ROOMS_PER_USER;
    let router = build_router(AppState::new(pool.clone(), config));

    TestApp { router, pool }
}

/// Build an app whose room events fan out through Redis Pub/Sub instead of the
/// in-memory channels. `redis` must already be connected; the pool is shared so
/// two instances can simulate separate processes over one database.
pub fn test_app_with_redis(pool: PgPool, redis: live_chat_server::ws::RedisPubSub) -> TestApp {
    let mut config = Config::from_env();
    config.jwt_secret = "integration-test-secret".to_string();
    config.max_rooms_per_user = MAX_ROOMS_PER_USER;
    let router = build_router(AppState::with_redis(pool.clone(), config, redis));

    TestApp { router, pool }
}

/// A username that passes validation and is unique per call.
pub fn unique_username() -> String {
    format!("u{}", &Uuid::new_v4().simple().to_string()[..12])
}

/// The strong default password used by tests.
pub const PASSWORD: &str = "correct horse battery staple";

/// Room cap pinned for tests, independent of any ambient `MAX_ROOMS_PER_USER`.
pub const MAX_ROOMS_PER_USER: i64 = 5;

/// Send a JSON request through the router and decode the JSON response.
pub async fn request(
    app: &TestApp,
    method: &str,
    uri: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = match body {
        Some(value) => Body::from(serde_json::to_vec(&value).unwrap()),
        None => Body::empty(),
    };

    let response = app
        .router
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .expect("router call failed");

    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("failed to read body")
        .to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

/// Register a fresh user and return `(token, username)`.
pub async fn register_user(app: &TestApp) -> (String, String) {
    let username = unique_username();
    let (status, body) = request(
        app,
        "POST",
        "/api/auth/register",
        Some(serde_json::json!({ "username": username, "password": PASSWORD })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "register failed: {body}");
    let token = body["token"].as_str().expect("token missing").to_string();
    (token, username)
}
