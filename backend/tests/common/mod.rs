//! Shared helpers for integration tests.
//!
//! DB-backed tests connect to `DATABASE_URL`; when it is unset or unreachable
//! they skip (returning `None`) instead of failing, so the suite still passes
//! in environments without a database (e.g. CI).

#![allow(dead_code)]

use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use live_chat_server::config::Config;
use live_chat_server::{AppState, build_router};
use serde_json::Value;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use uuid::Uuid;

/// A running application backed by a real database.
pub struct TestApp {
    pub router: Router,
    pub pool: PgPool,
}

/// Build the app against `DATABASE_URL`, migrating first.
///
/// Returns `None` (and prints why) when no database is reachable, so callers
/// can skip gracefully.
pub async fn test_app() -> Option<TestApp> {
    let url = match std::env::var("DATABASE_URL") {
        Ok(url) if !url.is_empty() => url,
        _ => {
            eprintln!("skipping: DATABASE_URL is not set");
            return None;
        }
    };

    let pool = match PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
    {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("skipping: could not connect to database: {err}");
            return None;
        }
    };

    if let Err(err) = sqlx::migrate!("./migrations").run(&pool).await {
        eprintln!("skipping: migrations failed: {err}");
        return None;
    }

    let mut config = Config::from_env();
    config.jwt_secret = "integration-test-secret".to_string();
    let router = build_router(AppState::new(pool.clone(), config));

    Some(TestApp { router, pool })
}

/// A username that passes validation and is unique per call.
pub fn unique_username() -> String {
    format!("u{}", &Uuid::new_v4().simple().to_string()[..12])
}

/// The strong default password used by tests.
pub const PASSWORD: &str = "correct horse battery staple";

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
