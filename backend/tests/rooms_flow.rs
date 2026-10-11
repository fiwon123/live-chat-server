//! End-to-end room, membership, and history flow over the real router.
//!
//! These tests need a database; when none is reachable they skip.

mod common;

use axum::http::StatusCode;
use common::{register_user, request, test_app};

#[tokio::test]
async fn create_room_auto_joins_creator() {
    let Some(app) = test_app().await else {
        return;
    };
    let (token, _) = register_user(&app).await;

    let (status, body) = request(
        &app,
        "POST",
        "/api/rooms",
        Some(serde_json::json!({ "name": "general" })),
        Some(&token),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert_eq!(body["name"], "general");
    assert_eq!(body["member"], true);
    assert!(body["id"].as_str().is_some());
}

#[tokio::test]
async fn create_room_rejects_empty_and_long_names() {
    let Some(app) = test_app().await else {
        return;
    };
    let (token, _) = register_user(&app).await;

    let (empty, _) = request(
        &app,
        "POST",
        "/api/rooms",
        Some(serde_json::json!({ "name": "   " })),
        Some(&token),
    )
    .await;
    let (long, _) = request(
        &app,
        "POST",
        "/api/rooms",
        Some(serde_json::json!({ "name": "x".repeat(65) })),
        Some(&token),
    )
    .await;

    assert_eq!(empty, StatusCode::BAD_REQUEST);
    assert_eq!(long, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn rooms_require_auth() {
    let Some(app) = test_app().await else {
        return;
    };
    let (status, _) = request(
        &app,
        "POST",
        "/api/rooms",
        Some(serde_json::json!({ "name": "general" })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn list_rooms_shows_membership_flag() {
    let Some(app) = test_app().await else {
        return;
    };
    let (token, _) = register_user(&app).await;
    let (other, _) = register_user(&app).await;

    let (status, body) = request(
        &app,
        "POST",
        "/api/rooms",
        Some(serde_json::json!({ "name": "shared" })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let room_id = body["id"].as_str().unwrap().to_string();

    // Creator sees member=true.
    let (status, list) = request(&app, "GET", "/api/rooms", None, Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let mine = list
        .as_array()
        .unwrap()
        .iter()
        .find(|room| room["id"] == room_id)
        .expect("creator should see the room");
    assert_eq!(mine["member"], true);

    // Non-member sees member=false.
    let (_, list) = request(&app, "GET", "/api/rooms", None, Some(&other)).await;
    let theirs = list
        .as_array()
        .unwrap()
        .iter()
        .find(|room| room["id"] == room_id)
        .expect("other user should see the room");
    assert_eq!(theirs["member"], false);
}

#[tokio::test]
async fn get_room_and_404() {
    let Some(app) = test_app().await else {
        return;
    };
    let (token, _) = register_user(&app).await;
    let (status, body) = request(
        &app,
        "POST",
        "/api/rooms",
        Some(serde_json::json!({ "name": "solo" })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let room_id = body["id"].as_str().unwrap().to_string();

    let (status, room) = request(
        &app,
        "GET",
        &format!("/api/rooms/{room_id}"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(room["name"], "solo");
    assert_eq!(room["member"], true);

    let (status, _) = request(
        &app,
        "GET",
        "/api/rooms/00000000-0000-0000-0000-000000000000",
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn join_and_leave_are_idempotent() {
    let Some(app) = test_app().await else {
        return;
    };
    let (token, _) = register_user(&app).await;
    let (joiner, _) = register_user(&app).await;

    let (status, body) = request(
        &app,
        "POST",
        "/api/rooms",
        Some(serde_json::json!({ "name": "town" })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let room_id = body["id"].as_str().unwrap().to_string();

    // Join twice -> both 204.
    for _ in 0..2 {
        let (status, _) = request(
            &app,
            "POST",
            &format!("/api/rooms/{room_id}/join"),
            None,
            Some(&joiner),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    // Member flag flips after join.
    let (_, list) = request(&app, "GET", "/api/rooms", None, Some(&joiner)).await;
    let joined = list
        .as_array()
        .unwrap()
        .iter()
        .find(|room| room["id"] == room_id)
        .unwrap();
    assert_eq!(joined["member"], true);

    // Leave twice -> both 204.
    for _ in 0..2 {
        let (status, _) = request(
            &app,
            "POST",
            &format!("/api/rooms/{room_id}/leave"),
            None,
            Some(&joiner),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    let (_, list) = request(&app, "GET", "/api/rooms", None, Some(&joiner)).await;
    let left = list
        .as_array()
        .unwrap()
        .iter()
        .find(|room| room["id"] == room_id)
        .unwrap();
    assert_eq!(left["member"], false);
}

#[tokio::test]
async fn history_requires_membership() {
    let Some(app) = test_app().await else {
        return;
    };
    let (token, _) = register_user(&app).await;
    let (outsider, _) = register_user(&app).await;

    // A members-only room: join is open by design, so an outsider who has not
    // joined is the interesting case here.
    let (status, body) = request(
        &app,
        "POST",
        "/api/rooms",
        Some(serde_json::json!({ "name": "members-only" })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let room_id = body["id"].as_str().unwrap().to_string();

    let (status, _) = request(
        &app,
        "GET",
        &format!("/api/rooms/{room_id}/messages"),
        None,
        Some(&outsider),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _) = request(
        &app,
        "GET",
        "/api/rooms/00000000-0000-0000-0000-000000000000/messages",
        None,
        Some(&outsider),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn history_returns_seeded_messages_with_authors() {
    let Some(app) = test_app().await else {
        return;
    };
    let (token, username) = register_user(&app).await;

    let (status, body) = request(
        &app,
        "POST",
        "/api/rooms",
        Some(serde_json::json!({ "name": "history" })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let room_id = body["id"].as_str().unwrap().to_string();
    let room_id_uuid = uuid::Uuid::parse_str(&room_id).unwrap();
    let user_id = body["created_by"].as_str().unwrap();

    // Seed messages directly through the pool (there is no send endpoint yet;
    // the WebSocket issue adds it). Runtime query, not the compile-time macro,
    // so the offline sqlx cache is unaffected. Explicit, spaced timestamps keep
    // the ordering deterministic regardless of how close the inserts land.
    let pool = &app.pool;
    for i in 0..3 {
        let created_at = chrono::Utc::now() - chrono::Duration::seconds((2 - i) as i64);
        sqlx::query(
            "INSERT INTO messages (room_id, user_id, body, created_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(room_id_uuid)
        .bind(uuid::Uuid::parse_str(user_id).unwrap())
        .bind(format!("hello {i}"))
        .bind(created_at)
        .execute(pool)
        .await
        .unwrap();
    }

    let (status, messages) = request(
        &app,
        "GET",
        &format!("/api/rooms/{room_id}/messages?limit=2"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let messages = messages.as_array().unwrap();
    assert_eq!(messages.len(), 2, "limit should cap the page");
    for message in messages {
        assert_eq!(message["username"], username);
        assert!(message["id"].as_str().is_some());
        assert!(message["created_at"].as_str().is_some());
    }
    // Newest first: body of the first page entry is "hello 2".
    assert_eq!(messages[0]["body"], "hello 2");

    // Paginate: fetch the rest with the oldest returned id as cursor.
    let cursor = messages[1]["id"].as_str().unwrap();
    let (status, older) = request(
        &app,
        "GET",
        &format!("/api/rooms/{room_id}/messages?limit=2&before={cursor}"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let older = older.as_array().unwrap();
    assert_eq!(older.len(), 1);
    assert_eq!(older[0]["body"], "hello 0");
}

#[tokio::test]
async fn unknown_cursor_yields_empty_page() {
    let Some(app) = test_app().await else {
        return;
    };
    let (token, _) = register_user(&app).await;
    let (status, body) = request(
        &app,
        "POST",
        "/api/rooms",
        Some(serde_json::json!({ "name": "pag" })),
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let room_id = body["id"].as_str().unwrap().to_string();

    // A well-formed cursor id that does not belong to any message yields an
    // empty page (the cursor join is scoped to the room).
    let cursor = "00000000-0000-0000-0000-000000000000";
    let (status, body) = request(
        &app,
        "GET",
        &format!("/api/rooms/{room_id}/messages?before={cursor}"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 0);
}
