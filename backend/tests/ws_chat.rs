//! WebSocket chat flow over a real socket: broadcast isolation, presence,
//! validation errors, and the save-before-broadcast guarantee.
//!
//! The server is spawned on an ephemeral port (`127.0.0.1:0`). Tests skip
//! gracefully when no database is reachable.

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

/// Build the app and serve it on `127.0.0.1:0`; `None` when no DB is reachable.
async fn spawn_server() -> Option<RunningServer> {
    let app = test_app().await?;
    let listener = TcpListener::bind("127.0.0.1:0").await.ok()?;
    let addr = listener.local_addr().ok()?;
    let router = app.router.clone();
    // Detached: the task lives until the test runtime shuts down.
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    Some(RunningServer { addr, app })
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

#[tokio::test]
async fn handshake_rejects_bad_token_unknown_room_and_non_member() {
    let Some(server) = spawn_server().await else {
        return;
    };
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

#[tokio::test]
async fn broadcasts_only_within_room_and_persists() {
    let Some(server) = spawn_server().await else {
        return;
    };

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

    let mut alice = connect_ws(&server, &alice_token, &room_a).await;
    let mut bob = connect_ws(&server, &bob_token, &room_a).await;
    let mut carol = connect_ws(&server, &carol_token, &room_b).await;

    // Everyone sees their own join; Alice also sees Bob's join arrive.
    let alice_first = next_event_of_type(&mut alice, "presence").await;
    assert_eq!(alice_first["event"], "joined");
    assert_eq!(alice_first["user"]["username"], alice_name);
    let bob_first = next_event_of_type(&mut bob, "presence").await;
    assert_eq!(bob_first["event"], "joined");
    assert_eq!(bob_first["user"]["username"], bob_name);
    let _ = next_event_of_type(&mut carol, "presence").await;
    let alice_second = next_event_of_type(&mut alice, "presence").await;
    assert_eq!(alice_second["user"]["username"], bob_name);

    // Bob disconnects; Alice gets the "left" presence event.
    drop(bob);
    let left = next_event_of_type(&mut alice, "presence").await;
    assert_eq!(left["event"], "left");
    assert_eq!(left["user"]["username"], bob_name);

    // Alice is still connected; re-attach Bob.
    let mut bob = connect_ws(&server, &bob_token, &room_a).await;
    let _ = next_event_of_type(&mut bob, "presence").await;
    let _ = next_event_of_type(&mut alice, "presence").await;

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

#[tokio::test]
async fn invalid_sends_get_error_events_but_connection_survives() {
    let Some(server) = spawn_server().await else {
        return;
    };
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
