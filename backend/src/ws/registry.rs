//! In-memory per-room broadcast registry (single server instance).
//!
//! Each room gets a `tokio::sync::broadcast` channel created lazily, plus a
//! map of connected user ids -> usernames used to emit presence events.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::RwLock;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::ws::{UserRef, WsOutgoing};

/// Outbound channel capacity per room; 256 buffered events is plenty for a
/// single-server demo and absorbs short stalls in slow clients.
const ROOM_CHANNEL_CAPACITY: usize = 256;

/// Cloneable handle to the whole registry.
#[derive(Clone, Default)]
pub struct RoomRegistry {
    rooms: Arc<RwLock<HashMap<Uuid, Arc<RoomState>>>>,
}

struct RoomState {
    tx: broadcast::Sender<WsOutgoing>,
    users: Mutex<HashMap<Uuid, String>>,
}

impl RoomRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribe to a room's event stream, creating the room lazily.
    pub async fn subscribe(&self, room_id: Uuid) -> broadcast::Receiver<WsOutgoing> {
        self.room(room_id).await.tx.subscribe()
    }

    /// Announce a user joining the room (no-op when the user is already
    /// connected, but the event is still emitted for duplicate connections).
    pub async fn join(&self, room_id: Uuid, user_id: Uuid, username: String) {
        let room = self.room(room_id).await;
        room.users.lock().unwrap().insert(user_id, username.clone());
        let _ = room.tx.send(WsOutgoing::Presence {
            event: "joined",
            user: UserRef {
                id: user_id,
                username,
            },
        });
    }

    /// Announce a user leaving; emits nothing if they were not connected.
    pub async fn leave(&self, room_id: Uuid, user_id: Uuid) {
        let Some(room) = self.rooms.read().await.get(&room_id).cloned() else {
            return;
        };
        let removed = room.users.lock().unwrap().remove(&user_id);
        if let Some(username) = removed {
            let _ = room.tx.send(WsOutgoing::Presence {
                event: "left",
                user: UserRef {
                    id: user_id,
                    username,
                },
            });
        }
    }

    /// Broadcast an event to everyone subscribed to the room.
    pub async fn send(&self, room_id: Uuid, event: WsOutgoing) {
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
