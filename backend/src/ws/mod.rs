//! Real-time chat: the WebSocket session loop and the in-memory per-room
//! broadcast registry.
//!
//! Single-instance by design: room connections and broadcast channels live in
//! memory only. Multi-instance fan-out via Redis Pub/Sub is a later milestone.

pub mod registry;

pub use registry::RoomRegistry;

use std::time::{Duration, Instant};

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
use crate::storage::reactions::MAX_EMOJI_LEN;
use crate::storage::{Messages, Reactions, RoomMemberships};

/// Maximum length of a chat message body (in chars, after trimming).
pub const MAX_MESSAGE_LEN: usize = 4000;

/// Minimum gap between relayed `typing` events from one session. Typing is
/// ephemeral (never persisted), so a chatty client should not be able to flood
/// the room channel; the client throttles too, and this is the server's
/// backstop.
const TYPING_THROTTLE: Duration = Duration::from_millis(1500);

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
    /// A user is composing a message (ephemeral; not persisted).
    Typing { user: UserRef },
    /// A reaction was added (`added: true`) or removed from a message.
    Reaction {
        message_id: Uuid,
        emoji: String,
        user_id: Uuid,
        username: String,
        added: bool,
    },
    /// A member marked messages read up to `message_id` (ephemeral receipt).
    Read { message_id: Uuid, user: UserRef },
    /// A protocol or validation error; the connection stays open.
    Error { message: String },
}

/// A received client frame.
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Incoming {
    Send {
        body: String,
    },
    /// The client is composing. No payload: the server knows the sender.
    Typing,
    /// Toggle a reaction on a message.
    React {
        message_id: Uuid,
        emoji: String,
    },
}

/// Everything one connected client needs to handle inbound frames.
struct SessionCtx {
    room_id: Uuid,
    user_id: Uuid,
    username: String,
    messages: Messages,
    memberships: RoomMemberships,
    reactions: Reactions,
    registry: RoomRegistry,
    direct_tx: mpsc::Sender<WsOutgoing>,
    /// When this session last relayed a `typing` event (throttle gate).
    typing_last: Option<Instant>,
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

    // Room-access is enforced on the receive path too: a session whose
    // membership has been revoked (e.g. the user left the room over HTTP) is
    // closed the moment the next message arrives, instead of silently
    // receiving room content forever.
    let send_memberships = RoomMemberships::new(pool.clone());
    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                event = room_rx.recv() => match event {
                    Ok(event @ WsOutgoing::Message { .. }) => {
                        match send_memberships.is_member(room_id, user_id).await {
                            Ok(true) => {}
                            Ok(false) => break, // membership revoked: close the session
                            Err(err) => {
                                tracing::error!(error = ?err, "receive-side membership check failed");
                                break;
                            }
                        }
                        if send_json(&mut sender, &event).await.is_err() { break; }
                    }
                    Ok(event) => {
                        // A user does not need their own typing echoed back.
                        if matches!(&event, WsOutgoing::Typing { user } if user.id == user_id) {
                            continue;
                        }
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
        let mut ctx = SessionCtx {
            room_id,
            user_id,
            username: username.clone(),
            messages: Messages::new(pool.clone()),
            memberships: RoomMemberships::new(pool.clone()),
            reactions: Reactions::new(pool),
            registry: recv_registry,
            direct_tx,
            typing_last: None,
        };

        while let Some(Ok(frame)) = receiver.next().await {
            match frame {
                WsMessage::Text(text) => handle_text(text.as_str(), &mut ctx).await,
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

/// Handle one inbound text frame: `{"type":"send","body":"..."}`,
/// `{"type":"typing"}`, or `{"type":"react","message_id":"...","emoji":"..."}`.
///
/// A failed frame never kills the connection: the client gets an `error`
/// event and keeps talking. Save happens before broadcast, so every receiver
/// (including the sender) sees the same id and timestamp.
async fn handle_text(text: &str, ctx: &mut SessionCtx) {
    let error = match parse_incoming(text) {
        Err(_) => Some(WsOutgoing::Error {
            message: "malformed message: expected a send, typing, or react frame".to_string(),
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
                                        reactions: Vec::new(),
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
        Ok(Incoming::Typing) => {
            handle_typing(ctx).await;
            None
        }
        Ok(Incoming::React { message_id, emoji }) => {
            handle_react(ctx, message_id, emoji).await;
            None
        }
    };

    if let Some(error) = error {
        // Errors are advisory; never let a full direct lane stall the receive
        // loop (a client that does not read has no use for them anyway).
        if ctx.direct_tx.try_send(error).is_err() {
            tracing::debug!(room_id = %ctx.room_id, "ws client error lane full; dropping error event");
        }
    }
}

/// Relay an ephemeral `typing` event to the room, membership-checked and
/// throttled per session. Never persisted. The gate runs before the membership
/// query so a client spamming `typing` cannot force a DB round-trip per frame.
async fn handle_typing(ctx: &mut SessionCtx) {
    let now = Instant::now();
    if let Some(last) = ctx.typing_last
        && now.duration_since(last) < TYPING_THROTTLE
    {
        return; // Throttled: silently drop, the room already knows.
    }

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

    ctx.typing_last = Some(now);

    ctx.registry
        .send(
            ctx.room_id,
            WsOutgoing::Typing {
                user: UserRef {
                    id: ctx.user_id,
                    username: ctx.username.clone(),
                },
            },
        )
        .await;
}

async fn send_error(direct_tx: &mpsc::Sender<WsOutgoing>, message: &str) {
    let _ = direct_tx.try_send(WsOutgoing::Error {
        message: message.to_string(),
    });
}

/// Toggle a reaction on a message in this room and broadcast the change.
///
/// Membership is re-checked, the message must belong to this session's room,
/// and the emoji is validated; any failure yields a non-fatal `error` event.
async fn handle_react(ctx: &mut SessionCtx, message_id: Uuid, emoji: String) {
    let emoji = match validate_emoji(&emoji) {
        Ok(emoji) => emoji,
        Err(err) => return send_error(&ctx.direct_tx, &err.to_string()).await,
    };

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

    // The target message must exist and live in this room.
    match ctx.messages.find_by_id(message_id).await {
        Ok(Some(message)) if message.room_id == ctx.room_id => {}
        Ok(_) => return send_error(&ctx.direct_tx, "message not found in this room").await,
        Err(err) => {
            tracing::error!(error = ?err, "message lookup failed");
            return send_error(&ctx.direct_tx, "failed to load message").await;
        }
    }

    let added = match ctx.reactions.toggle(message_id, ctx.user_id, &emoji).await {
        Ok(added) => added,
        Err(err) => {
            tracing::error!(error = ?err, "failed to toggle reaction");
            return send_error(&ctx.direct_tx, "failed to save reaction").await;
        }
    };

    ctx.registry
        .send(
            ctx.room_id,
            WsOutgoing::Reaction {
                message_id,
                emoji,
                user_id: ctx.user_id,
                username: ctx.username.clone(),
                added,
            },
        )
        .await;
}

fn parse_incoming(text: &str) -> Result<Incoming, serde_json::Error> {
    serde_json::from_str(text)
}

/// Validate a message body: trimmed, non-empty, at most [`MAX_MESSAGE_LEN`],
/// no control characters (mirrors room-name validation: embedded NULs must
/// not reach Postgres).
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
    if body.chars().any(char::is_control) {
        return Err(AppError::BadRequest(
            "message must not contain control characters".to_string(),
        ));
    }
    Ok(body)
}

/// Validate a reaction emoji: trimmed, non-empty, at most [`MAX_EMOJI_LEN`]
/// chars, no control characters.
fn validate_emoji(emoji: &str) -> Result<String, AppError> {
    let emoji = emoji.trim();
    if emoji.is_empty() {
        return Err(AppError::BadRequest("emoji must not be empty".to_string()));
    }
    if emoji.chars().count() > MAX_EMOJI_LEN {
        return Err(AppError::BadRequest(format!(
            "emoji must be at most {MAX_EMOJI_LEN} characters"
        )));
    }
    if emoji.chars().any(char::is_control) {
        return Err(AppError::BadRequest(
            "emoji must not contain control characters".to_string(),
        ));
    }
    Ok(emoji.to_string())
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
    use super::{validate_body, validate_emoji};

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

    #[test]
    fn rejects_control_characters() {
        assert!(validate_body("hello\u{0}world").is_err());
        assert!(validate_body("tab\there").is_err());
    }

    #[test]
    fn accepts_a_single_emoji() {
        assert_eq!(validate_emoji("  👍  ").unwrap(), "👍");
        // Skin-tone + ZWJ sequences stay intact (not trimmed or split).
        assert_eq!(validate_emoji("👍🏽").unwrap(), "👍🏽");
    }

    #[test]
    fn rejects_empty_or_overlong_emoji() {
        assert!(validate_emoji("   ").is_err());
        assert!(validate_emoji("").is_err());
        let too_long = "x".repeat(super::MAX_EMOJI_LEN + 1);
        assert!(validate_emoji(&too_long).is_err());
    }

    #[test]
    fn rejects_emoji_with_control_characters() {
        assert!(validate_emoji("a\u{0}b").is_err());
    }
}
