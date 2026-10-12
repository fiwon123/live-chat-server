//! Cross-instance broadcasting over Redis Pub/Sub: two in-process server
//! instances share one database and one Redis; a message sent into a room on
//! instance A is delivered to a client connected to instance B (and vice
//! versa), exactly once, without the local channel double-delivering.
//!
//! Requires a reachable Redis (`REDIS_URL`); skips gracefully otherwise, like
//! the DB-backed tests.

mod common;

use std::net::SocketAddr;
use std::time::Duration;

use axum::http::StatusCode;
use common::{TestApp, register_user, request, test_app_with_redis};
use futures_util::{SinkExt, StreamExt};
use live_chat_server::ws::RedisPubSub;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// A live server on an ephemeral port plus the router/pool for HTTP calls.
struct RunningServer {
    addr: SocketAddr,
    app: TestApp,
}

/// Build a Redis-backed app and serve it on `127.0.0.1:0`; `None` when no DB
/// is reachable.
async fn spawn_server(redis: RedisPubSub) -> Option<RunningServer> {
    let app = test_app_with_redis(redis).await?;
    let listener = TcpListener::bind("127.0.0.1:0").await.ok()?;
    let addr = listener.local_addr().ok()?;
    let router = app.router.clone();
    // Detached: the task lives until the test runtime shuts down.
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    Some(RunningServer { addr, app })
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

/// Assert that no event arrives within `within` (proves no double delivery).
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
async fn redis_pubsub_fans_out_across_instances_without_double_delivery() {
    let url = match std::env::var("REDIS_URL") {
        Ok(url) if !url.is_empty() => url,
        _ => {
            eprintln!("skipping: REDIS_URL is not set (no Redis to test against)");
            return;
        }
    };

    // Each instance gets its own connection, like two separate processes.
    let redis_a = match RedisPubSub::connect(&url).await {
        Ok(redis) => redis,
        Err(err) => {
            eprintln!("skipping: redis unreachable: {err}");
            return;
        }
    };
    let redis_b = match RedisPubSub::connect(&url).await {
        Ok(redis) => redis,
        Err(err) => {
            eprintln!("skipping: redis unreachable: {err}");
            return;
        }
    };

    let Some(server_a) = spawn_server(redis_a).await else {
        return;
    };
    let Some(server_b) = spawn_server(redis_b).await else {
        return;
    };

    // Alice registers and creates the room on instance A; Bob registers and
    // joins through instance B (both hit the same database).
    let (alice_token, alice_name) = register_user(&server_a.app).await;
    let (bob_token, bob_name) = register_user(&server_b.app).await;

    let (status, body) = request(
        &server_a.app,
        "POST",
        "/api/rooms",
        Some(json!({ "name": "pubsub-room" })),
        Some(&alice_token),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create room failed: {body}");
    let room_id = body["id"].as_str().unwrap().to_string();

    let (status, _) = request(
        &server_b.app,
        "POST",
        &format!("/api/rooms/{room_id}/join"),
        None,
        Some(&bob_token),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Alice connects to instance A, Bob to instance B.
    let mut alice = connect_ws(&server_a, &alice_token, &room_id).await;
    let alice_joined = next_event_of_type(&mut alice, "presence").await;
    assert_eq!(alice_joined["event"], "joined");
    assert_eq!(alice_joined["user"]["username"], alice_name);

    let mut bob = connect_ws(&server_b, &bob_token, &room_id).await;
    let bob_joined = next_event_of_type(&mut bob, "presence").await;
    assert_eq!(bob_joined["event"], "joined");
    assert_eq!(bob_joined["user"]["username"], bob_name);

    // Bob's join crossed instances: Alice, on instance A, sees it.
    let alice_sees_bob = next_event_of_type(&mut alice, "presence").await;
    assert_eq!(alice_sees_bob["event"], "joined");
    assert_eq!(alice_sees_bob["user"]["username"], bob_name);

    // A message sent on instance A reaches Bob on instance B...
    send_text(
        &mut alice,
        r#"{"type":"send","body":"across the instances"}"#,
    )
    .await;

    let alice_msg = next_event_of_type(&mut alice, "message").await;
    assert_eq!(alice_msg["message"]["body"], "across the instances");
    let bob_msg = next_event_of_type(&mut bob, "message").await;
    assert_eq!(bob_msg["message"]["body"], "across the instances");
    assert_eq!(
        bob_msg["message"]["id"], alice_msg["message"]["id"],
        "both instances must deliver the identical persisted message"
    );

    // ...and a message sent on instance B reaches Alice on instance A.
    send_text(&mut bob, r#"{"type":"send","body":"back the other way"}"#).await;
    let bob_msg = next_event_of_type(&mut bob, "message").await;
    assert_eq!(bob_msg["message"]["body"], "back the other way");
    let alice_msg = next_event_of_type(&mut alice, "message").await;
    assert_eq!(alice_msg["message"]["body"], "back the other way");

    // No double delivery: with both streams drained, nothing else arrives on
    // either (publish-only mode never writes the local in-memory channel, so
    // each instance delivers exactly one copy per message).
    assert_no_event(&mut alice, Duration::from_millis(300)).await;
    assert_no_event(&mut bob, Duration::from_millis(300)).await;

    drop(alice);
    drop(bob);

    // Presence and messages were persisted as usual: history has both bodies.
    let (status, history) = request(
        &server_a.app,
        "GET",
        &format!("/api/rooms/{room_id}/messages"),
        None,
        Some(&alice_token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let history = history.as_array().unwrap();
    let bodies: Vec<&str> = history
        .iter()
        .map(|m| m["body"].as_str().unwrap())
        .collect();
    assert!(bodies.contains(&"across the instances"), "{bodies:?}");
    assert!(bodies.contains(&"back the other way"), "{bodies:?}");
}
