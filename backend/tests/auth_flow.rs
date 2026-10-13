//! End-to-end auth flow over the real router.
//!
//! Each test runs against its own isolated database (created and dropped by
//! `#[sqlx::test]`), so the suite never touches the development database and
//! can assert on global state. A reachable `DATABASE_URL` is required.

mod common;

use axum::http::StatusCode;
use common::{PASSWORD, register_user, request, test_app, unique_username};

#[sqlx::test]
async fn register_returns_token_and_public_user(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let username = unique_username();
    let (status, body) = request(
        &app,
        "POST",
        "/api/auth/register",
        Some(serde_json::json!({ "username": username, "password": PASSWORD })),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(body["token"].as_str().is_some_and(|t| !t.is_empty()));
    assert_eq!(body["user"]["username"], username);
    assert!(body["user"]["id"].as_str().is_some());
    assert!(
        body["user"].get("password_hash").is_none(),
        "password hash must never be serialized"
    );
}

#[sqlx::test]
async fn duplicate_username_is_rejected(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let username = unique_username();
    let payload = serde_json::json!({ "username": username, "password": PASSWORD });

    let (first, _) = request(
        &app,
        "POST",
        "/api/auth/register",
        Some(payload.clone()),
        None,
    )
    .await;
    let (second, body) = request(&app, "POST", "/api/auth/register", Some(payload), None).await;

    assert_eq!(first, StatusCode::CREATED);
    assert_eq!(second, StatusCode::CONFLICT);
    assert!(body["error"].as_str().is_some());
}

#[sqlx::test]
async fn register_rejects_invalid_username(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (status, _) = request(
        &app,
        "POST",
        "/api/auth/register",
        Some(serde_json::json!({ "username": "No", "password": PASSWORD })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn register_rejects_short_password(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let username = unique_username();
    let (status, _) = request(
        &app,
        "POST",
        "/api/auth/register",
        Some(serde_json::json!({ "username": username, "password": "short" })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn login_succeeds_with_correct_password(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (_, username) = register_user(&app).await;

    let (status, body) = request(
        &app,
        "POST",
        "/api/auth/login",
        Some(serde_json::json!({ "username": username, "password": PASSWORD })),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["token"].as_str().is_some_and(|t| !t.is_empty()));
    assert_eq!(body["user"]["username"], username);
}

#[sqlx::test]
async fn login_rejects_wrong_password(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (_, username) = register_user(&app).await;

    let (status, _) = request(
        &app,
        "POST",
        "/api/auth/login",
        Some(serde_json::json!({ "username": username, "password": "not-the-password" })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn login_rejects_unknown_user_with_same_status(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (status, _) = request(
        &app,
        "POST",
        "/api/auth/login",
        Some(serde_json::json!({ "username": unique_username(), "password": PASSWORD })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn me_returns_current_user(pool: sqlx::PgPool) {
    let app = test_app(pool);
    let (token, username) = register_user(&app).await;

    let (status, body) = request(&app, "GET", "/api/me", None, Some(&token)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["username"], username);
    assert!(body.get("password_hash").is_none());
}

#[sqlx::test]
async fn me_requires_a_valid_token(pool: sqlx::PgPool) {
    let app = test_app(pool);

    let (missing, _) = request(&app, "GET", "/api/me", None, None).await;
    let (bad, _) = request(&app, "GET", "/api/me", None, Some("not.a.jwt")).await;

    assert_eq!(missing, StatusCode::UNAUTHORIZED);
    assert_eq!(bad, StatusCode::UNAUTHORIZED);
}
