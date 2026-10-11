//! End-to-end auth flow over the real router.
//!
//! These tests need a database; when none is reachable they skip.

mod common;

use axum::http::StatusCode;
use common::{PASSWORD, register_user, request, test_app, unique_username};

#[tokio::test]
async fn register_returns_token_and_public_user() {
    let Some(app) = test_app().await else {
        return;
    };
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

#[tokio::test]
async fn duplicate_username_is_rejected() {
    let Some(app) = test_app().await else {
        return;
    };
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

#[tokio::test]
async fn register_rejects_invalid_username() {
    let Some(app) = test_app().await else {
        return;
    };
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

#[tokio::test]
async fn register_rejects_short_password() {
    let Some(app) = test_app().await else {
        return;
    };
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

#[tokio::test]
async fn login_succeeds_with_correct_password() {
    let Some(app) = test_app().await else {
        return;
    };
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

#[tokio::test]
async fn login_rejects_wrong_password() {
    let Some(app) = test_app().await else {
        return;
    };
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

#[tokio::test]
async fn login_rejects_unknown_user_with_same_status() {
    let Some(app) = test_app().await else {
        return;
    };
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

#[tokio::test]
async fn me_returns_current_user() {
    let Some(app) = test_app().await else {
        return;
    };
    let (token, username) = register_user(&app).await;

    let (status, body) = request(&app, "GET", "/api/me", None, Some(&token)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["username"], username);
    assert!(body.get("password_hash").is_none());
}

#[tokio::test]
async fn me_requires_a_valid_token() {
    let Some(app) = test_app().await else {
        return;
    };

    let (missing, _) = request(&app, "GET", "/api/me", None, None).await;
    let (bad, _) = request(&app, "GET", "/api/me", None, Some("not.a.jwt")).await;

    assert_eq!(missing, StatusCode::UNAUTHORIZED);
    assert_eq!(bad, StatusCode::UNAUTHORIZED);
}
