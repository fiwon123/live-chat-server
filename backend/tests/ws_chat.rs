//! WebSocket chat flow over a real socket: broadcast isolation, presence,
//! validation errors, revoked-membership teardown, and the save-before-
//! broadcast guarantee.
//!
//! Each test runs against its own isolated database (created and dropped by
//! `#[sqlx::test]`); the server is spawned on an ephemeral port
//! (`127.0.0.1:0`).

mod common;

use std::net::SocketAddr;
use std::time::Duration;

use axum::http::StatusCode;
use common::{TestApp, register_user, request, test_app};
use futures_util::{SinkExt, StreamExt};
use live_chat_server::ws::MAX_MESSAGE_LEN;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// A live server on an ephemeral port plus the router/pool for HTTP calls.
struct RunningServer {
    addr: SocketAddr,
    app: TestApp,
}

/// Build the app and serve it on `127.0.0.1:0`.
async fn spawn_server(pool: sqlx::PgPool) -> RunningServer {
    let app = test_app(pool);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("read local addr");
    let router = app.router.clone();
    // Detached: the task lives until the test runtime shuts down.
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    RunningServer { addr, app }
}

/// Create a room as the given user; returns its id and creator's user id.
async fn create_room(app: &TestApp, token: &str, name: &str) -> (String, String) {
    let (status, body) = request(
        app,
        "POST",
        "/api/rooms",
        Some(json!({ "name": name })),
        Some(token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create room failed: {body}");
    let room_id = body["id"].as_str().unwrap().to_string();
    let creator = body["created_by"].as_str().unwrap().to_string();
    (room_id, creator)
}

async fn connect_ws(server: &RunningServer, token: &str, room_id: &str) -> WsStream {
    let url = format!(
        "ws://{}/api/ws?token={token}&room_id={room_id}",
        server.addr
    );
    let (stream, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("ws handshake failed");
    stream
}

/// Attempt a real socket handshake; returns the response status plus the
/// stream when the upgrade was accepted.
async fn connect_ws_status(
    server: &RunningServer,
    token: &str,
    room_id: &str,
) -> (StatusCode, Option<WsStream>) {
    let url = format!(
        "ws://{}/api/ws?token={token}&room_id={room_id}",
        server.addr
    );
    match tokio_tungstenite::connect_async(&url).await {
        Ok((stream, _)) => (StatusCode::SWITCHING_PROTOCOLS, Some(stream)),
        Err(tokio_tungstenite::tungstenite::Error::Http(response)) => (response.status(), None),
        Err(err) => panic!("ws connect failed: {err}"),
    }
}

async fn send_text(stream: &mut WsStream, text: &str) {
    stream
        .send(Message::Text(text.to_string().into()))
        .await
        .expect("ws send failed");
}

/// Read the next text frame as JSON; panics on timeout/close/protocol errors.
async fn next_event(stream: &mut WsStream) -> Value {
    let frame = tokio::time::timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("timed out waiting for ws event")
        .expect("ws stream ended")
        .expect("ws frame error");
    match frame {
        Message::Text(text) => serde_json::from_str(&text).expect("event is not JSON"),
        other => panic!("expected text frame, got: {other:?}"),
    }
}

/// Skip ahead to the next event of the given `type` (drops earlier ones).
async fn next_event_of_type(stream: &mut WsStream, ty: &str) -> Value {
    loop {
        let event = next_event(stream).await;
        if event["type"] == ty {
            return event;
        }
    }
}

/// Assert that no event arrives within `within` (proves room isolation).
async fn assert_no_event(stream: &mut WsStream, within: Duration) {
    match tokio::time::timeout(within, stream.next()).await {
        Err(_) => {}
        Ok(None) => {}
        Ok(Some(Ok(Message::Text(text)))) => panic!("unexpected event: {text}"),
        Ok(Some(Ok(other))) => panic!("unexpected frame: {other:?}"),
        Ok(Some(Err(err))) => panic!("ws error: {err}"),
    }
}

/// Assert the remote end closed the connection (session torn down).
async fn assert_stream_closed(stream: &mut WsStream) {
    match tokio::time::timeout(Duration::from_secs(5), stream.next()).await {
        Ok(None) => {}
        Ok(Some(Ok(Message::Close(_)))) => {}
        Ok(Some(Err(_))) => {}
        other => panic!("expected session close, got: {other:?}"),
    }
}

#[sqlx::test]
async fn handshake_rejects_bad_token_unknown_room_and_non_member(pool: sqlx::PgPool) {
    let server = spawn_server(pool).await;
    let (token, _) = register_user(&server.app).await;
    let (outsider, _) = register_user(&server.app).await;

    let (room_id, _) = create_room(&server.app, &token, "ws-only").await;
    let unknown = Uuid::new_v4().to_string();

    // Invalid token -> 401 before any room checks.
    let (status, stream) = connect_ws_status(&server, "bogus", &room_id).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(stream.is_none());

    // Valid token, unknown room -> 404.
    let (status, stream) = connect_ws_status(&server, &token, &unknown).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(stream.is_none());

    // Registered user who never joined -> 403.
    let (status, stream) = connect_ws_status(&server, &outsider, &room_id).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(stream.is_none());

    // Member -> upgrade accepted (101 Switching Protocols).
    let (status, stream) = connect_ws_status(&server, &token, &room_id).await;
    assert_eq!(status, StatusCode::SWITCHING_PROTOCOLS);
    drop(stream);
}

#[sqlx::test]
async fn broadcasts_only_within_room_and_persists(pool: sqlx::PgPool) {
    let server = spawn_server(pool).await;

    // Alice creates room A (creator is a member), Bob joins it.
    // Carol creates room B and is its only member.
    let (alice_token, alice_name) = register_user(&server.app).await;
    let (bob_token, bob_name) = register_user(&server.app).await;
    let (carol_token, _) = register_user(&server.app).await;

    let (room_a, alice_id) = create_room(&server.app, &alice_token, "room-a").await;
    let (room_b, _) = create_room(&server.app, &carol_token, "room-b").await;

    let (status, _) = request(
        &server.app,
        "POST",
        &format!("/api/rooms/{room_a}/join"),
        None,
        Some(&bob_token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Connect one client at a time and wait for its own `joined` presence —
    // that is the server's signal that subscribe+join completed, so later
    // connections cannot be missed (no cross-client ordering race).
    let mut alice = connect_ws(&server, &alice_token, &room_a).await;
    let alice_joined = next_event_of_type(&mut alice, "presence").await;
    assert_eq!(alice_joined["event"], "joined");
    assert_eq!(alice_joined["user"]["username"], alice_name);

    let mut bob = connect_ws(&server, &bob_token, &room_a).await;
    let bob_joined = next_event_of_type(&mut bob, "presence").await;
    assert_eq!(bob_joined["event"], "joined");
    assert_eq!(bob_joined["user"]["username"], bob_name);
    let alice_sees_bob = next_event_of_type(&mut alice, "presence").await;
    assert_eq!(alice_sees_bob["event"], "joined");
    assert_eq!(alice_sees_bob["user"]["username"], bob_name);

    let mut carol = connect_ws(&server, &carol_token, &room_b).await;
    let carol_joined = next_event_of_type(&mut carol, "presence").await;
    assert_eq!(carol_joined["event"], "joined");

    // Bob disconnects; Alice gets the "left" presence event.
    drop(bob);
    let bob_left = next_event_of_type(&mut alice, "presence").await;
    assert_eq!(bob_left["event"], "left");
    assert_eq!(bob_left["user"]["username"], bob_name);

    // Bob reconnects; presence transition goes joined again.
    let mut bob = connect_ws(&server, &bob_token, &room_a).await;
    let _ = next_event_of_type(&mut bob, "presence").await;
    let bob_rejoined = next_event_of_type(&mut alice, "presence").await;
    assert_eq!(bob_rejoined["event"], "joined");
    assert_eq!(bob_rejoined["user"]["username"], bob_name);

    // Alice sends; both room-A clients get the identical persisted message.
    send_text(&mut alice, r#"{"type":"send","body":"hello from room A"}"#).await;

    for stream in [&mut alice, &mut bob] {
        let event = next_event_of_type(stream, "message").await;
        let message = &event["message"];
        assert_eq!(message["body"], "hello from room A");
        assert_eq!(message["room_id"], room_a);
        assert_eq!(message["user_id"], alice_id);
        assert_eq!(message["username"], alice_name);
        assert!(message["id"].as_str().is_some(), "message must carry an id");
        assert!(
            message["created_at"].as_str().is_some(),
            "message must carry a timestamp"
        );
    }

    // Carol (different room) must not receive anything.
    assert_no_event(&mut carol, Duration::from_millis(300)).await;

    // The message was persisted before broadcast: history agrees.
    let (status, history) = request(
        &server.app,
        "GET",
        &format!("/api/rooms/{room_a}/messages"),
        None,
        Some(&alice_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let history = history.as_array().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0]["body"], "hello from room A");
    assert_eq!(history[0]["username"], alice_name);
}

#[sqlx::test]
async fn typing_broadcasts_within_room_only_and_is_not_persisted(pool: sqlx::PgPool) {
    let server = spawn_server(pool).await;

    let (alice_token, alice_name) = register_user(&server.app).await;
    let (bob_token, bob_name) = register_user(&server.app).await;
    let (carol_token, _) = register_user(&server.app).await;

    let (room_a, alice_id) = create_room(&server.app, &alice_token, "typing-a").await;
    let (room_b, _) = create_room(&server.app, &carol_token, "typing-b").await;

    let (status, _) = request(
        &server.app,
        "POST",
        &format!("/api/rooms/{room_a}/join"),
        None,
        Some(&bob_token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Drain each client's own `joined` so the streams are clear.
    let mut alice = connect_ws(&server, &alice_token, &room_a).await;
    let _ = next_event_of_type(&mut alice, "presence").await;
    let mut bob = connect_ws(&server, &bob_token, &room_a).await;
    let _ = next_event_of_type(&mut bob, "presence").await;
    let _ = next_event_of_type(&mut alice, "presence").await;
    let mut carol = connect_ws(&server, &carol_token, &room_b).await;
    let _ = next_event_of_type(&mut carol, "presence").await;

    // Alice starts typing; Bob (same room) sees it with her identity.
    send_text(&mut alice, r#"{"type":"typing"}"#).await;
    let typing = next_event_of_type(&mut bob, "typing").await;
    assert_eq!(typing["user"]["username"], alice_name);
    assert_eq!(typing["user"]["id"], alice_id);

    // A second typing frame within the throttle window is dropped server-side
    // (sent immediately, so well inside the 1.5 s gate even on a slow machine).
    send_text(&mut alice, r#"{"type":"typing"}"#).await;
    assert_no_event(&mut bob, Duration::from_millis(300)).await;

    // The sender never receives its own typing event.
    assert_no_event(&mut alice, Duration::from_millis(300)).await;

    // Carol (different room) must not see it.
    assert_no_event(&mut carol, Duration::from_millis(300)).await;

    // The connection survives the typing frames: a real message still flows.
    let expected = format!("hello {bob_name}");
    send_text(
        &mut alice,
        &format!(r#"{{"type":"send","body":"{expected}"}}"#),
    )
    .await;
    let event = next_event_of_type(&mut bob, "message").await;
    assert_eq!(event["message"]["body"], expected);

    // Typing was never persisted: history holds only the real message.
    let (_, history) = request(
        &server.app,
        "GET",
        &format!("/api/rooms/{room_a}/messages"),
        None,
        Some(&alice_token),
    )
    .await;
    let history = history.as_array().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0]["body"], expected);
}

#[sqlx::test]
async fn reactions_toggle_persist_and_broadcast_within_room_only(pool: sqlx::PgPool) {
    let server = spawn_server(pool).await;

    let (alice_token, _alice_name) = register_user(&server.app).await;
    let (bob_token, bob_name) = register_user(&server.app).await;
    let (dave_token, _) = register_user(&server.app).await;
    let (carol_token, _) = register_user(&server.app).await;

    let (room_a, _) = create_room(&server.app, &alice_token, "react-a").await;
    let (room_b, _) = create_room(&server.app, &carol_token, "react-b").await;

    // Bob and Dave join room A; Carol owns room B.
    for token in [&bob_token, &dave_token] {
        let (status, _) = request(
            &server.app,
            "POST",
            &format!("/api/rooms/{room_a}/join"),
            None,
            Some(token),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
    }

    // Connect one at a time, draining each client's own `joined` so the
    // streams are clear before the interesting events.
    let mut alice = connect_ws(&server, &alice_token, &room_a).await;
    let _ = next_event_of_type(&mut alice, "presence").await;
    let mut bob = connect_ws(&server, &bob_token, &room_a).await;
    let bob_joined = next_event_of_type(&mut bob, "presence").await;
    let bob_id = bob_joined["user"]["id"].as_str().unwrap().to_string();
    let _ = next_event_of_type(&mut alice, "presence").await;
    let mut dave = connect_ws(&server, &dave_token, &room_a).await;
    let dave_joined = next_event_of_type(&mut dave, "presence").await;
    let dave_id = dave_joined["user"]["id"].as_str().unwrap().to_string();
    // Alice and Bob see Dave join.
    let _ = next_event_of_type(&mut alice, "presence").await;
    let _ = next_event_of_type(&mut bob, "presence").await;
    let mut carol = connect_ws(&server, &carol_token, &room_b).await;
    let _ = next_event_of_type(&mut carol, "presence").await;

    // Alice sends a message; capture its id from both room-A clients.
    send_text(&mut alice, r#"{"type":"send","body":"reactable"}"#).await;
    let alice_msg = next_event_of_type(&mut alice, "message").await;
    let bob_msg = next_event_of_type(&mut bob, "message").await;
    let _ = next_event_of_type(&mut dave, "message").await;
    let message_id = alice_msg["message"]["id"].as_str().unwrap().to_string();
    assert_eq!(bob_msg["message"]["id"], message_id);
    // A freshly persisted message carries no reactions.
    assert!(
        alice_msg["message"]["reactions"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    // Reacting to your OWN message is rejected; nothing is broadcast.
    send_text(
        &mut alice,
        &json!({ "type": "react", "message_id": message_id, "emoji": "👍" }).to_string(),
    )
    .await;
    let error = next_event_of_type(&mut alice, "error").await;
    assert!(
        error["message"].as_str().unwrap().contains("your own"),
        "error message: {}",
        error["message"]
    );
    assert_no_event(&mut bob, Duration::from_millis(200)).await;
    assert_no_event(&mut dave, Duration::from_millis(200)).await;

    // Bob reacts 👍 to Alice's message: all room-A clients (including the
    // sender) see it.
    send_text(
        &mut bob,
        &json!({ "type": "react", "message_id": message_id, "emoji": "👍" }).to_string(),
    )
    .await;
    for stream in [&mut alice, &mut bob, &mut dave] {
        let event = next_event_of_type(stream, "reaction").await;
        assert_eq!(event["message_id"], message_id);
        assert_eq!(event["emoji"], "👍");
        assert_eq!(event["user_id"], bob_id);
        assert_eq!(event["username"], bob_name);
        assert_eq!(event["added"], true);
    }

    // Carol (different room) must not see the reaction.
    assert_no_event(&mut carol, Duration::from_millis(300)).await;

    // History reflects the persisted reaction.
    let (status, history) = request(
        &server.app,
        "GET",
        &format!("/api/rooms/{room_a}/messages"),
        None,
        Some(&alice_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let history = history.as_array().unwrap();
    let reactions = history[0]["reactions"].as_array().unwrap();
    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0]["emoji"], "👍");
    assert_eq!(reactions[0]["count"], 1);
    assert_eq!(reactions[0]["user_ids"][0], bob_id);

    // Dave adds the same emoji: still one group, but count 2.
    send_text(
        &mut dave,
        &json!({ "type": "react", "message_id": message_id, "emoji": "👍" }).to_string(),
    )
    .await;
    for stream in [&mut alice, &mut bob, &mut dave] {
        let event = next_event_of_type(stream, "reaction").await;
        assert_eq!(event["user_id"], dave_id);
        assert_eq!(event["added"], true);
    }
    let (_, history) = request(
        &server.app,
        "GET",
        &format!("/api/rooms/{room_a}/messages"),
        None,
        Some(&alice_token),
    )
    .await;
    let reactions = history[0]["reactions"].as_array().unwrap();
    assert_eq!(
        reactions.len(),
        1,
        "same emoji must not create a second group"
    );
    assert_eq!(reactions[0]["count"], 2);

    // Bob reacts 👍 again: toggles his reaction off (added: false); Dave's stays.
    send_text(
        &mut bob,
        &json!({ "type": "react", "message_id": message_id, "emoji": "👍" }).to_string(),
    )
    .await;
    for stream in [&mut alice, &mut bob, &mut dave] {
        let event = next_event_of_type(stream, "reaction").await;
        assert_eq!(event["user_id"], bob_id);
        assert_eq!(event["added"], false);
    }
    let (_, history) = request(
        &server.app,
        "GET",
        &format!("/api/rooms/{room_a}/messages"),
        None,
        Some(&alice_token),
    )
    .await;
    let reactions = history[0]["reactions"].as_array().unwrap();
    assert_eq!(reactions.len(), 1);
    assert_eq!(reactions[0]["count"], 1);
    assert_eq!(reactions[0]["user_ids"][0], dave_id);

    // Reacting to a message in another room is rejected without delivery.
    send_text(&mut carol, r#"{"type":"send","body":"in room b"}"#).await;
    let carol_msg = next_event_of_type(&mut carol, "message").await;
    let other_message_id = carol_msg["message"]["id"].as_str().unwrap().to_string();
    send_text(
        &mut alice,
        &json!({ "type": "react", "message_id": other_message_id, "emoji": "👍" }).to_string(),
    )
    .await;
    let error = next_event_of_type(&mut alice, "error").await;
    assert!(
        error["message"].as_str().unwrap().contains("not found"),
        "error message: {}",
        error["message"]
    );
    assert_no_event(&mut bob, Duration::from_millis(300)).await;

    // An empty emoji is rejected too.
    send_text(
        &mut alice,
        &json!({ "type": "react", "message_id": message_id, "emoji": "   " }).to_string(),
    )
    .await;
    let error = next_event_of_type(&mut alice, "error").await;
    assert!(error["message"].as_str().unwrap().contains("empty"));

    // The connection survived all failures: a valid message still flows.
    send_text(&mut alice, r#"{"type":"send","body":"still here"}"#).await;
    let event = next_event_of_type(&mut bob, "message").await;
    assert_eq!(event["message"]["body"], "still here");

    // Only the two valid messages were persisted; the second has no reactions.
    let (_, history) = request(
        &server.app,
        "GET",
        &format!("/api/rooms/{room_a}/messages"),
        None,
        Some(&alice_token),
    )
    .await;
    let history = history.as_array().unwrap();
    assert_eq!(history.len(), 2);
    // History is newest-first.
    assert_eq!(history[0]["body"], "still here");
    assert!(history[0]["reactions"].as_array().unwrap().is_empty());
    assert_eq!(history[1]["body"], "reactable");
    assert_eq!(history[1]["reactions"][0]["count"], 1);
}

#[sqlx::test]
async fn invalid_sends_get_error_events_but_connection_survives(pool: sqlx::PgPool) {
    let server = spawn_server(pool).await;
    let (token, _) = register_user(&server.app).await;
    let (room_id, _) = create_room(&server.app, &token, "validate").await;

    let mut stream = connect_ws(&server, &token, &room_id).await;
    let _ = next_event_of_type(&mut stream, "presence").await;

    // Blank body (whitespace only) -> error, nothing persisted.
    send_text(&mut stream, r#"{"type":"send","body":"   "}"#).await;
    let error = next_event_of_type(&mut stream, "error").await;
    assert!(error["message"].as_str().unwrap().contains("empty"));

    // Over-length body -> error.
    let too_long = json!({ "type": "send", "body": "x".repeat(MAX_MESSAGE_LEN + 1) }).to_string();
    send_text(&mut stream, &too_long).await;
    let error = next_event_of_type(&mut stream, "error").await;
    assert!(
        error["message"].as_str().unwrap().contains("4000"),
        "error message: {}",
        error["message"]
    );

    // Malformed JSON -> error.
    send_text(&mut stream, "this is not json").await;
    let error = next_event_of_type(&mut stream, "error").await;
    assert!(error["message"].as_str().unwrap().contains("malformed"));

    // The connection survived all three failures: a valid send works.
    send_text(&mut stream, r#"{"type":"send","body":"still here"}"#).await;
    let event = next_event_of_type(&mut stream, "message").await;
    assert_eq!(event["message"]["body"], "still here");

    // Only the valid message was persisted; error frames never touch the DB.
    let (_, history) = request(
        &server.app,
        "GET",
        &format!("/api/rooms/{room_id}/messages"),
        None,
        Some(&token),
    )
    .await;
    let history = history.as_array().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0]["body"], "still here");
}

#[sqlx::test]
async fn multiline_messages_broadcast_and_persist_with_normalized_newlines(pool: sqlx::PgPool) {
    let server = spawn_server(pool).await;
    let (token, _) = register_user(&server.app).await;
    let (room_id, _) = create_room(&server.app, &token, "multiline").await;

    let mut stream = connect_ws(&server, &token, &room_id).await;
    let _ = next_event_of_type(&mut stream, "presence").await;

    // A message with real `\n` newlines is accepted body-for-body.
    send_text(
        &mut stream,
        &json!({"type": "send", "body": "line one\nline two"}).to_string(),
    )
    .await;
    let event = next_event_of_type(&mut stream, "message").await;
    assert_eq!(event["message"]["body"], "line one\nline two");

    // CRLF and lone CR are normalized to LF so any client's line endings land intact.
    send_text(
        &mut stream,
        &json!({"type": "send", "body": "a\r\nb\r\nc"}).to_string(),
    )
    .await;
    let event = next_event_of_type(&mut stream, "message").await;
    assert_eq!(event["message"]["body"], "a\nb\nc");

    send_text(
        &mut stream,
        &json!({"type": "send", "body": "d\re"}).to_string(),
    )
    .await;
    let event = next_event_of_type(&mut stream, "message").await;
    assert_eq!(event["message"]["body"], "d\ne");

    // Other control characters are still rejected (NUL must not reach Postgres).
    send_text(
        &mut stream,
        &json!({"type": "send", "body": "bad\u{0}nul"}).to_string(),
    )
    .await;
    let error = next_event_of_type(&mut stream, "error").await;
    assert!(
        error["message"].as_str().unwrap().contains("control"),
        "error message: {}",
        error["message"]
    );

    // All three valid messages persisted with their (normalized) newlines.
    let (status, history) = request(
        &server.app,
        "GET",
        &format!("/api/rooms/{room_id}/messages"),
        None,
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let history = history.as_array().unwrap();
    assert_eq!(history.len(), 3);
    // History is newest-first and ties break on a random UUID, so compare the
    // persisted bodies order-independently: all three, newlines intact.
    let mut bodies: Vec<&str> = history
        .iter()
        .map(|m| m["body"].as_str().unwrap())
        .collect();
    bodies.sort_unstable();
    assert_eq!(bodies, vec!["a\nb\nc", "d\ne", "line one\nline two"]);
}

#[sqlx::test]
async fn revoked_membership_closes_session_without_delivery(pool: sqlx::PgPool) {
    let server = spawn_server(pool).await;
    let (alice_token, alice_name) = register_user(&server.app).await;
    let (bob_token, _) = register_user(&server.app).await;

    let (room_a, _) = create_room(&server.app, &alice_token, "room-a").await;
    let (status, _) = request(
        &server.app,
        "POST",
        &format!("/api/rooms/{room_a}/join"),
        None,
        Some(&bob_token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let mut alice = connect_ws(&server, &alice_token, &room_a).await;
    let _ = next_event_of_type(&mut alice, "presence").await;
    let mut bob = connect_ws(&server, &bob_token, &room_a).await;
    let _ = next_event_of_type(&mut bob, "presence").await;
    let _ = next_event_of_type(&mut alice, "presence").await;

    // Alice leaves the room over HTTP while her socket stays open.
    let (status, _) = request(
        &server.app,
        "POST",
        &format!("/api/rooms/{room_a}/leave"),
        None,
        Some(&alice_token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Bob sends: it reaches Bob (still a member)...
    send_text(&mut bob, r#"{"type":"send","body":"after leave"}"#).await;
    let event = next_event_of_type(&mut bob, "message").await;
    assert_eq!(event["message"]["body"], "after leave");

    // ...but Alice's session is torn down WITHOUT ever delivering it.
    assert_stream_closed(&mut alice).await;

    // Bob observes Alice's session end via presence.
    let left = next_event_of_type(&mut bob, "presence").await;
    assert_eq!(left["event"], "left");
    assert_eq!(left["user"]["username"], alice_name);
}

#[sqlx::test]
async fn duplicate_connections_reference_count_presence(pool: sqlx::PgPool) {
    let server = spawn_server(pool).await;
    let (observer_token, observer_name) = register_user(&server.app).await;
    let (y_token, y_name) = register_user(&server.app).await;

    let (room_id, _) = create_room(&server.app, &observer_token, "dup").await;
    let (status, _) = request(
        &server.app,
        "POST",
        &format!("/api/rooms/{room_id}/join"),
        None,
        Some(&y_token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let mut observer = connect_ws(&server, &observer_token, &room_id).await;
    let joined = next_event_of_type(&mut observer, "presence").await;
    assert_eq!(joined["user"]["username"], observer_name);

    // Y opens a first connection: one `joined`.
    let mut y1 = connect_ws(&server, &y_token, &room_id).await;
    let joined = next_event_of_type(&mut y1, "presence").await;
    assert_eq!(joined["event"], "joined");
    assert_eq!(joined["user"]["username"], y_name);
    let _ = next_event_of_type(&mut observer, "presence").await;

    // Y opens a second connection: no duplicate `joined` anywhere.
    let mut y2 = connect_ws(&server, &y_token, &room_id).await;
    assert_no_event(&mut y2, Duration::from_millis(300)).await;
    assert_no_event(&mut observer, Duration::from_millis(300)).await;

    // A message proves the second connection is live and subscribed.
    send_text(&mut y1, r#"{"type":"send","body":"both tabs"}"#).await;
    for stream in [&mut y1, &mut y2] {
        let event = next_event_of_type(stream, "message").await;
        assert_eq!(event["message"]["body"], "both tabs");
    }
    let _ = next_event_of_type(&mut observer, "message").await;

    // Closing the first connection must not emit a premature `left`.
    drop(y1);
    assert_no_event(&mut observer, Duration::from_millis(300)).await;

    // Closing the last connection emits exactly one `left`.
    drop(y2);
    let left = next_event_of_type(&mut observer, "presence").await;
    assert_eq!(left["event"], "left");
    assert_eq!(left["user"]["username"], y_name);
}

#[sqlx::test]
async fn read_receipts_broadcast_within_room_only(pool: sqlx::PgPool) {
    let server = spawn_server(pool).await;

    let (alice_token, _) = register_user(&server.app).await;
    let (bob_token, bob_name) = register_user(&server.app).await;
    let (carol_token, _) = register_user(&server.app).await;

    let (room_a, _) = create_room(&server.app, &alice_token, "read-a").await;
    let (room_b, _) = create_room(&server.app, &carol_token, "read-b").await;

    let (status, _) = request(
        &server.app,
        "POST",
        &format!("/api/rooms/{room_a}/join"),
        None,
        Some(&bob_token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Drain each client's own `joined` so the streams are clear.
    let mut alice = connect_ws(&server, &alice_token, &room_a).await;
    let _ = next_event_of_type(&mut alice, "presence").await;
    let mut bob = connect_ws(&server, &bob_token, &room_a).await;
    let bob_joined = next_event_of_type(&mut bob, "presence").await;
    let bob_id = bob_joined["user"]["id"].as_str().unwrap().to_string();
    let _ = next_event_of_type(&mut alice, "presence").await;
    let mut carol = connect_ws(&server, &carol_token, &room_b).await;
    let _ = next_event_of_type(&mut carol, "presence").await;

    // Alice sends a message; capture its id.
    send_text(&mut alice, r#"{"type":"send","body":"read me"}"#).await;
    let alice_msg = next_event_of_type(&mut alice, "message").await;
    let _ = next_event_of_type(&mut bob, "message").await;
    let message_id = alice_msg["message"]["id"].as_str().unwrap().to_string();

    // Bob marks the room read over HTTP; Alice sees the receipt.
    let (status, _) = request(
        &server.app,
        "POST",
        &format!("/api/rooms/{room_a}/read"),
        Some(json!({ "message_id": message_id })),
        Some(&bob_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let read = next_event_of_type(&mut alice, "read").await;
    assert_eq!(read["message_id"], message_id);
    assert_eq!(read["user"]["id"], bob_id);
    assert_eq!(read["user"]["username"], bob_name);

    // Carol (different room) must not see the receipt.
    assert_no_event(&mut carol, Duration::from_millis(300)).await;
}

#[sqlx::test]
async fn deleting_a_room_notifies_members_and_no_one_else(pool: sqlx::PgPool) {
    let server = spawn_server(pool).await;

    let (alice_token, _) = register_user(&server.app).await;
    let (bob_token, _) = register_user(&server.app).await;
    let (carol_token, _) = register_user(&server.app).await;

    let (room_a, _) = create_room(&server.app, &alice_token, "delete-a").await;
    let (room_b, _) = create_room(&server.app, &carol_token, "delete-b").await;

    let (status, _) = request(
        &server.app,
        "POST",
        &format!("/api/rooms/{room_a}/join"),
        None,
        Some(&bob_token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Drain each client's own `joined` so the streams are clear.
    let mut alice = connect_ws(&server, &alice_token, &room_a).await;
    let _ = next_event_of_type(&mut alice, "presence").await;
    let mut bob = connect_ws(&server, &bob_token, &room_a).await;
    let _ = next_event_of_type(&mut bob, "presence").await;
    let _ = next_event_of_type(&mut alice, "presence").await;
    let mut carol = connect_ws(&server, &carol_token, &room_b).await;
    let _ = next_event_of_type(&mut carol, "presence").await;

    // The owner deletes room A over HTTP.
    let (status, _) = request(
        &server.app,
        "DELETE",
        &format!("/api/rooms/{room_a}"),
        None,
        Some(&alice_token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Every member — the owner included — is told the room is gone.
    for stream in [&mut alice, &mut bob] {
        let event = next_event_of_type(stream, "room_deleted").await;
        assert_eq!(event["room_id"], room_a);
    }

    // A client in another room is unaffected.
    assert_no_event(&mut carol, Duration::from_millis(300)).await;

    // The room is really gone: history now 404s for a (former) member.
    let (status, _) = request(
        &server.app,
        "GET",
        &format!("/api/rooms/{room_a}/messages"),
        None,
        Some(&bob_token),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
