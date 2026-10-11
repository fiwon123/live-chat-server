//! Real-time chat: the WebSocket session loop and the in-memory per-room
//! broadcast registry.
//!
//! Single-instance by design: room connections and broadcast channels live in
//! memory only. Multi-instance fan-out via Redis Pub/Sub is a later milestone.

pub mod registry;

pub use registry::RoomRegistry;

use axum::Error as AxumError;
use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use sqlx::PgPool;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::error::AppError;
use crate::routes::rooms::MessageResponse;
use crate::storage::{Messages, RoomMemberships};

/// Maximum length of a chat message body (in chars, after trimming).
pub const MAX_MESSAGE_LEN: usize = 4000;

/// A user reference embedded in presence events.
#[derive(Debug, Clone, Serialize)]
pub struct UserRef {
    pub id: Uuid,
    pub username: String,
}

/// Events the server sends to a client.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsOutgoing {
    /// A chat message persisted and broadcast to the room.
    Message { message: MessageResponse },
    /// A user joined or left the room.
    Presence { event: &'static str, user: UserRef },
    /// A protocol or validation error; the connection stays open.
    Error { message: String },
}

/// A received client frame.
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Incoming {
    Send { body: String },
}

/// Everything one connected client needs to handle inbound frames.
struct SessionCtx {
    room_id: Uuid,
    user_id: Uuid,
    username: String,
    messages: Messages,
    memberships: RoomMemberships,
    registry: RoomRegistry,
    direct_tx: mpsc::Sender<WsOutgoing>,
}

/// Run the socket session for one connection: read client frames, persist and
/// broadcast messages, and relay room events back to the client.
pub async fn run_session(
    socket: WebSocket,
    registry: RoomRegistry,
    pool: PgPool,
    room_id: Uuid,
    user_id: Uuid,
    username: String,
) {
    // Subscribe before announcing the join so no room event is missed.
    let mut room_rx = registry.subscribe(room_id).await;
    registry.join(room_id, user_id, username.clone()).await;

    let (mut sender, mut receiver) = socket.split();
    // Direct outbound lane for this client (errors) alongside the room
    // broadcast lane.
    let (direct_tx, mut direct_rx) = mpsc::channel::<WsOutgoing>(32);

    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                event = room_rx.recv() => match event {
                    Ok(event) => {
                        if send_json(&mut sender, &event).await.is_err() { break; }
                    }
                    Err(RecvError::Lagged(skipped)) => {
                        tracing::warn!(room_id = %room_id, skipped, "ws client lagged behind broadcast");
                    }
                    Err(RecvError::Closed) => break,
                },
                Some(event) = direct_rx.recv() => {
                    if send_json(&mut sender, &event).await.is_err() { break; }
                }
            }
        }
    });

    let recv_registry = registry.clone();
    let mut recv_task = tokio::spawn(async move {
        let ctx = SessionCtx {
            room_id,
            user_id,
            username: username.clone(),
            messages: Messages::new(pool.clone()),
            memberships: RoomMemberships::new(pool),
            registry: recv_registry,
            direct_tx,
        };

        while let Some(Ok(frame)) = receiver.next().await {
            match frame {
                WsMessage::Text(text) => handle_text(text.as_str(), &ctx).await,
                WsMessage::Ping(_) | WsMessage::Pong(_) => {}
                WsMessage::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => {},
        _ = &mut recv_task => {},
    }

    recv_task.abort();
    send_task.abort();
    registry.leave(room_id, user_id).await;
    tracing::info!(room_id = %room_id, user_id = %user_id, "ws disconnected");
}

/// Handle one inbound text frame (`{"type": "send", "body": "..."}`).
///
/// A failed frame never kills the connection: the client gets an `error`
/// event and keeps talking. Save happens before broadcast, so every receiver
/// (including the sender) sees the same id and timestamp.
async fn handle_text(text: &str, ctx: &SessionCtx) {
    let error = match parse_incoming(text) {
        Err(_) => Some(WsOutgoing::Error {
            message: "malformed message: expected {\"type\":\"send\",\"body\":\"...\"}".to_string(),
        }),
        Ok(Incoming::Send { body }) => match validate_body(&body) {
            Err(err) => Some(WsOutgoing::Error {
                message: err.to_string(),
            }),
            Ok(body) => {
                // Room-access check on every action.
                let member = match ctx.memberships.is_member(ctx.room_id, ctx.user_id).await {
                    Ok(member) => member,
                    Err(err) => {
                        tracing::error!(error = ?err, "membership check failed");
                        return send_error(&ctx.direct_tx, "failed to verify membership").await;
                    }
                };
                if !member {
                    return send_error(&ctx.direct_tx, "you are not a member of this room").await;
                }

                match ctx.messages.insert(ctx.room_id, ctx.user_id, &body).await {
                    Ok(message) => {
                        ctx.registry
                            .send(
                                ctx.room_id,
                                WsOutgoing::Message {
                                    message: MessageResponse {
                                        id: message.id,
                                        room_id: ctx.room_id,
                                        user_id: ctx.user_id,
                                        username: ctx.username.clone(),
                                        body: message.body,
                                        created_at: message.created_at,
                                    },
                                },
                            )
                            .await;
                        None
                    }
                    Err(err) => {
                        tracing::error!(error = ?err, "failed to persist message");
                        Some(WsOutgoing::Error {
                            message: "failed to save message".to_string(),
                        })
                    }
                }
            }
        },
    };

    if let Some(error) = error {
        let _ = ctx.direct_tx.send(error).await;
    }
}

async fn send_error(direct_tx: &mpsc::Sender<WsOutgoing>, message: &str) {
    let _ = direct_tx
        .send(WsOutgoing::Error {
            message: message.to_string(),
        })
        .await;
}

fn parse_incoming(text: &str) -> Result<Incoming, serde_json::Error> {
    serde_json::from_str(text)
}

/// Validate a message body: trimmed, non-empty, at most [`MAX_MESSAGE_LEN`].
fn validate_body(body: &str) -> Result<String, AppError> {
    let body = body.trim().to_string();
    if body.is_empty() {
        return Err(AppError::BadRequest(
            "message must not be empty".to_string(),
        ));
    }
    if body.chars().count() > MAX_MESSAGE_LEN {
        return Err(AppError::BadRequest(format!(
            "message must be at most {MAX_MESSAGE_LEN} characters"
        )));
    }
    Ok(body)
}

async fn send_json(
    sender: &mut SplitSink<WebSocket, WsMessage>,
    event: &WsOutgoing,
) -> Result<(), AxumError> {
    let text = serde_json::to_string(event).map_err(AxumError::new)?;
    sender.send(WsMessage::Text(text.into())).await
}

#[cfg(test)]
mod tests {
    use super::validate_body;

    #[test]
    fn trims_and_accepts_body() {
        let body = validate_body("  hello world  ").unwrap();
        assert_eq!(body, "hello world");
    }

    #[test]
    fn rejects_empty_body() {
        assert!(validate_body("   ").is_err());
        assert!(validate_body("").is_err());
    }

    #[test]
    fn rejects_overlong_body() {
        let too_long = "x".repeat(super::MAX_MESSAGE_LEN + 1);
        assert!(validate_body(&too_long).is_err());
        let at_limit = "x".repeat(super::MAX_MESSAGE_LEN);
        assert!(validate_body(&at_limit).is_ok());
    }
}
