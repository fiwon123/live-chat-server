//! Per-room broadcast registry.
//!
//! Each room gets a `tokio::sync::broadcast` channel created lazily, plus a
//! reference count of connected users per user id used to emit presence
//! events. Presence is edge-triggered: `joined` on the first connection,
//! `left` when the last connection for a user disappears, so multiple tabs
//! from one user never produce spurious join/leave chatter.
//!
//! When Redis Pub/Sub is enabled the local channels are bypassed: `send`
//! publishes to `room:{uuid}`, and sessions receive events from their own
//! subscription instead of the in-memory channel. The user refcounts stay
//! per-instance, so presence is announced per instance (a documented
//! limitation of the single-server-first design).

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::{Arc, Mutex};

use tokio::sync::RwLock;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::ws::{RedisPubSub, RoomSubscriber, UserRef, WsOutgoing};

/// Outbound channel capacity per room; 256 buffered events is plenty for a
/// single-server demo and absorbs short stalls in slow clients.
const ROOM_CHANNEL_CAPACITY: usize = 256;

/// Cloneable handle to the whole registry.
#[derive(Clone)]
pub struct RoomRegistry {
    rooms: Arc<RwLock<HashMap<Uuid, Arc<RoomState>>>>,
    /// When set, room events fan out through Redis Pub/Sub instead of the
    /// in-memory channels.
    redis: Option<RedisPubSub>,
}

struct RoomState {
    tx: broadcast::Sender<WsOutgoing>,
    /// user_id -> (username, number of open connections).
    users: Mutex<HashMap<Uuid, (String, usize)>>,
}

impl RoomRegistry {
    /// A purely in-memory registry (single server instance).
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
            redis: None,
        }
    }

    /// A registry that fans room events out through Redis Pub/Sub.
    pub fn with_redis(redis: RedisPubSub) -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
            redis: Some(redis),
        }
    }

    /// Subscribe to a room's event stream, creating the room lazily.
    pub async fn subscribe(&self, room_id: Uuid) -> broadcast::Receiver<WsOutgoing> {
        self.room(room_id).await.tx.subscribe()
    }

    /// The event stream a session should read: the Redis subscription when
    /// pub/sub is enabled, else the in-memory room channel.
    ///
    /// Subscribe happens before [`RoomRegistry::join`] so no room event is
    /// missed. When Redis is configured but the subscription fails, the
    /// session falls back to the local channel (nothing else on this instance
    /// publishes locally in that mode, so the session simply stays quiet until
    /// Redis is reachable again).
    pub async fn events(&self, room_id: Uuid) -> RoomEvents {
        if let Some(redis) = &self.redis {
            match redis.subscribe(room_id).await {
                Ok(subscriber) => return RoomEvents::PubSub(subscriber),
                Err(err) => {
                    tracing::error!(room_id = %room_id, error = ?err, "redis subscribe failed; falling back to in-memory channel");
                }
            }
        }
        RoomEvents::Local(self.subscribe(room_id).await)
    }

    /// Register a connection and announce `joined` on the first one only.
    pub async fn join(&self, room_id: Uuid, user_id: Uuid, username: String) {
        let room = self.room(room_id).await;
        let first_connection = {
            let mut users = room
                .users
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match users.entry(user_id) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().1 += 1;
                    false
                }
                Entry::Vacant(entry) => {
                    entry.insert((username.clone(), 1));
                    true
                }
            }
        };
        if first_connection {
            self.send(
                room_id,
                WsOutgoing::Presence {
                    event: "joined".to_string(),
                    user: UserRef {
                        id: user_id,
                        username,
                    },
                },
            )
            .await;
        }
    }

    /// Unregister a connection and announce `left` when the last one goes.
    pub async fn leave(&self, room_id: Uuid, user_id: Uuid) {
        let Some(room) = self.rooms.read().await.get(&room_id).cloned() else {
            return;
        };
        let last_connection = {
            let mut users = room
                .users
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            match users.entry(user_id) {
                Entry::Occupied(mut entry) if entry.get().1 > 1 => {
                    entry.get_mut().1 -= 1;
                    None
                }
                Entry::Occupied(entry) => {
                    let (username, _) = entry.remove();
                    Some(username)
                }
                Entry::Vacant(_) => None,
            }
        };
        if let Some(username) = last_connection {
            self.send(
                room_id,
                WsOutgoing::Presence {
                    event: "left".to_string(),
                    user: UserRef {
                        id: user_id,
                        username,
                    },
                },
            )
            .await;
        }
    }

    /// Broadcast an event to everyone subscribed to the room.
    pub async fn send(&self, room_id: Uuid, event: WsOutgoing) {
        if let Some(redis) = &self.redis {
            // Publish-only: every interested session on every instance (this
            // one included) receives the event via its own subscription, so
            // the local channel is never written in pub/sub mode (that would
            // double-deliver).
            if let Err(err) = redis.publish(room_id, event).await {
                tracing::error!(room_id = %room_id, error = ?err, "redis publish failed");
            }
            return;
        }
        if let Some(room) = self.rooms.read().await.get(&room_id) {
            // No receivers is not an error worth surfacing: the message is
            // already persisted, so late joiners get it from history.
            let _ = room.tx.send(event);
        }
    }

    async fn room(&self, room_id: Uuid) -> Arc<RoomState> {
        let mut rooms = self.rooms.write().await;
        rooms
            .entry(room_id)
            .or_insert_with(|| {
                Arc::new(RoomState {
                    tx: broadcast::channel(ROOM_CHANNEL_CAPACITY).0,
                    users: Mutex::new(HashMap::new()),
                })
            })
            .clone()
    }
}

impl Default for RoomRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// The event source a session reads room events from.
pub enum RoomEvents {
    /// In-memory per-room channel (single server instance).
    Local(broadcast::Receiver<WsOutgoing>),
    /// Cross-instance subscription over Redis Pub/Sub.
    PubSub(RoomSubscriber),
}

/// What a session's next room event turned into.
pub enum RecvOutcome {
    /// A normal room event to relay to the client.
    Event(WsOutgoing),
    /// The in-memory channel dropped buffered events (slow client).
    Lagged(u64),
    /// The source ended (room channel closed or Redis connection dropped):
    /// the session should close.
    Closed,
}

impl RoomEvents {
    /// Yield the next room event from whichever source is active.
    pub async fn recv(&mut self) -> RecvOutcome {
        match self {
            RoomEvents::Local(rx) => match rx.recv().await {
                Ok(event) => RecvOutcome::Event(event),
                Err(broadcast::error::RecvError::Lagged(skipped)) => RecvOutcome::Lagged(skipped),
                Err(broadcast::error::RecvError::Closed) => RecvOutcome::Closed,
            },
            RoomEvents::PubSub(subscriber) => match subscriber.next().await {
                Some(Ok(event)) => RecvOutcome::Event(event),
                Some(Err(err)) => {
                    tracing::error!(error = ?err, "redis subscription error; closing session");
                    RecvOutcome::Closed
                }
                None => RecvOutcome::Closed,
            },
        }
    }
}
