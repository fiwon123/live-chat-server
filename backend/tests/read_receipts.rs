//! Read-state flow over the real router: unread counts, cursor advancement,
//! membership gating, and the `POST /api/rooms/{id}/read` validations.
//!
//! These tests need a database; when none is reachable they skip.

mod common;

use axum::http::StatusCode;
use common::{PASSWORD, TestApp, request, test_app, unique_username};

/// Insert a message directly through the pool and return its id.
///
/// A runtime query (not the compile-time macro), so the offline sqlx cache is
/// unaffected. `secs_ago` gives explicit, spaced timestamps so ordering does not
/// depend on how close the inserts land.
async fn seed_message(
    app: &TestApp,
    room_id: &str,
    user_id: &str,
    body: &str,
    secs_ago: i64,
) -> String {
    let created_at = chrono::Utc::now() - chrono::Duration::seconds(secs_ago);
    let id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO messages (room_id, user_id, body, created_at) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(uuid::Uuid::parse_str(room_id).unwrap())
    .bind(uuid::Uuid::parse_str(user_id).unwrap())
    .bind(body)
    .bind(created_at)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    id.to_string()
}

/// Register a fresh user and return `(token, user_id)`.
async fn register_user_and_id(app: &TestApp) -> (String, String) {
    let (status, body) = request(
        app,
        "POST",
        "/api/auth/register",
        Some(serde_json::json!({ "username": unique_username(), "password": PASSWORD })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "register failed: {body}");
    (
        body["token"].as_str().expect("token missing").to_string(),
        body["user"]["id"]
            .as_str()
            .expect("user id missing")
            .to_string(),
    )
}

/// Create a room as `token`; returns `(room_id, creator_id)`.
async fn create_room(app: &TestApp, token: &str, name: &str) -> (String, String) {
    let (status, body) = request(
        app,
        "POST",
        "/api/rooms",
        Some(serde_json::json!({ "name": name })),
        Some(token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create room failed: {body}");
    (
        body["id"].as_str().unwrap().to_string(),
        body["created_by"].as_str().unwrap().to_string(),
    )
}

/// Fetch a room from `GET /api/rooms` for one user (list entry).
async fn room_from_list(app: &TestApp, token: &str, room_id: &str) -> serde_json::Value {
    let (status, list) = request(app, "GET", "/api/rooms", None, Some(token)).await;
    assert_eq!(status, StatusCode::OK);
    list.as_array()
        .unwrap()
        .iter()
        .find(|room| room["id"] == room_id)
        .cloned()
        .expect("room should be listed")
}

#[tokio::test]
async fn unread_count_tracks_cursor_and_new_messages() {
    let Some(app) = test_app().await else {
        return;
    };
    let (token, _) = register_user_and_id(&app).await;
    let (room_id, _) = create_room(&app, &token, "unread").await;
    let (_, author_id) = register_user_and_id(&app).await;

    // A freshly created room starts clean.
    let (status, room) = request(
        &app,
        "GET",
        &format!("/api/rooms/{room_id}"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(room["unread_count"], 0);
    assert!(room["last_message"].is_null());

    // Three messages from another user arrive: all unread, newest as preview.
    for i in 0..3 {
        seed_message(&app, &room_id, &author_id, &format!("m{i}"), 10 - i).await;
    }
    let listed = room_from_list(&app, &token, &room_id).await;
    assert_eq!(listed["unread_count"], 3);
    assert_eq!(listed["last_message"]["body"], "m2");
    assert_eq!(listed["last_message"]["user_id"], author_id);

    // Mark everything read; the endpoint reports the new cursor and zero unread.
    let (status, body) = request(
        &app,
        "POST",
        &format!("/api/rooms/{room_id}/read"),
        Some(serde_json::json!({})),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["room_id"], room_id);
    assert_eq!(body["unread_count"], 0);
    assert!(body["last_read_message_id"].as_str().is_some());

    let listed = room_from_list(&app, &token, &room_id).await;
    assert_eq!(listed["unread_count"], 0);

    // A new message makes exactly one unread and updates the preview.
    seed_message(&app, &room_id, &author_id, "m3", 0).await;
    let listed = room_from_list(&app, &token, &room_id).await;
    assert_eq!(listed["unread_count"], 1);
    assert_eq!(listed["last_message"]["body"], "m3");

    // The viewer's own messages never count as unread.
    let creator_id = room["created_by"].as_str().unwrap();
    seed_message(&app, &room_id, creator_id, "mine", 0).await;
    let listed = room_from_list(&app, &token, &room_id).await;
    assert_eq!(listed["unread_count"], 1, "own message must not count");
}

#[tokio::test]
async fn mark_read_at_a_specific_message_leaves_newer_unread() {
    let Some(app) = test_app().await else {
        return;
    };
    let (token, _) = register_user_and_id(&app).await;
    let (room_id, _) = create_room(&app, &token, "cursor").await;
    let (_, author_id) = register_user_and_id(&app).await;

    let mut ids = Vec::new();
    for i in 0..3 {
        ids.push(seed_message(&app, &room_id, &author_id, &format!("m{i}"), 10 - i).await);
    }

    // Advancing one message at a time decrements the remaining unread and
    // reports the effective cursor.
    for (index, id) in ids.iter().enumerate() {
        let (status, body) = request(
            &app,
            "POST",
            &format!("/api/rooms/{room_id}/read"),
            Some(serde_json::json!({ "message_id": id })),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let expected = (ids.len() - 1 - index) as i64;
        assert_eq!(
            body["unread_count"], expected,
            "after marking message {index} read"
        );
        let last_read = body["last_read_message_id"].as_str().unwrap();
        assert_eq!(
            last_read, id,
            "advanced cursor must report the marked message"
        );
    }

    // Marking an older message again never moves the cursor backwards: no
    // effective cursor, no broadcast-worthy change, unread stays zero.
    let (status, body) = request(
        &app,
        "POST",
        &format!("/api/rooms/{room_id}/read"),
        Some(serde_json::json!({ "message_id": ids[0] })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["unread_count"], 0);
    assert!(
        body["last_read_message_id"].is_null(),
        "stale mark must not report a cursor: {body}"
    );
}

#[tokio::test]
async fn mark_read_on_empty_room_is_a_noop() {
    let Some(app) = test_app().await else {
        return;
    };
    let (token, _) = register_user_and_id(&app).await;
    let (room_id, _) = create_room(&app, &token, "empty").await;

    let (status, body) = request(
        &app,
        "POST",
        &format!("/api/rooms/{room_id}/read"),
        Some(serde_json::json!({})),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["last_read_message_id"].is_null());
    assert_eq!(body["unread_count"], 0);
}

#[tokio::test]
async fn non_members_see_no_room_preview_or_unread() {
    let Some(app) = test_app().await else {
        return;
    };
    let (token, _) = register_user_and_id(&app).await;
    let (outsider, _) = register_user_and_id(&app).await;
    let (_, author_id) = register_user_and_id(&app).await;
    let (room_id, _) = create_room(&app, &token, "locked").await;

    seed_message(&app, &room_id, &author_id, "secret", 0).await;

    // The outsider never joined: the list shows the room but no preview
    // and no unread badge.
    let listed = room_from_list(&app, &outsider, &room_id).await;
    assert_eq!(listed["member"], false);
    assert_eq!(listed["unread_count"], 0);
    assert!(listed["last_message"].is_null());

    let (status, room) = request(
        &app,
        "GET",
        &format!("/api/rooms/{room_id}"),
        None,
        Some(&outsider),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(room["member"], false);
    assert_eq!(room["unread_count"], 0);
    assert!(room["last_message"].is_null());

    // The member still sees the preview and the unread badge.
    let listed = room_from_list(&app, &token, &room_id).await;
    assert_eq!(listed["member"], true);
    assert_eq!(listed["unread_count"], 1);
    assert_eq!(listed["last_message"]["body"], "secret");
}

#[tokio::test]
async fn mark_read_enforces_membership_and_room_existence() {
    let Some(app) = test_app().await else {
        return;
    };
    let (token, _) = register_user_and_id(&app).await;
    let (outsider, _) = register_user_and_id(&app).await;
    let (room_id, _) = create_room(&app, &token, "locked").await;

    // Unknown room -> 404 (before the membership check).
    let (status, _) = request(
        &app,
        "POST",
        "/api/rooms/00000000-0000-0000-0000-000000000000/read",
        Some(serde_json::json!({})),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Registered but never joined -> 403.
    let (status, _) = request(
        &app,
        "POST",
        &format!("/api/rooms/{room_id}/read"),
        Some(serde_json::json!({})),
        Some(&outsider),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn mark_read_rejects_a_message_from_another_room() {
    let Some(app) = test_app().await else {
        return;
    };
    let (token, _) = register_user_and_id(&app).await;
    let (room_a, user_id) = create_room(&app, &token, "room-a").await;
    let (room_b, _) = create_room(&app, &token, "room-b").await;

    let foreign = seed_message(&app, &room_b, &user_id, "elsewhere", 0).await;

    let (status, body) = request(
        &app,
        "POST",
        &format!("/api/rooms/{room_a}/read"),
        Some(serde_json::json!({ "message_id": foreign })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
}
