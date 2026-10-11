//! In-memory per-room broadcast registry (single server instance).
//!
//! Each room gets a `tokio::sync::broadcast` channel created lazily, plus a
//! reference count of connected users per user id used to emit presence
//! events. Presence is edge-triggered: `joined` on the first connection,
//! `left` when the last connection for a user disappears, so multiple tabs
//! from one user never produce spurious join/leave chatter.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
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
    /// user_id -> (username, number of open connections).
    users: Mutex<HashMap<Uuid, (String, usize)>>,
}

impl RoomRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribe to a room's event stream, creating the room lazily.
    pub async fn subscribe(&self, room_id: Uuid) -> broadcast::Receiver<WsOutgoing> {
        self.room(room_id).await.tx.subscribe()
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
            let _ = room.tx.send(WsOutgoing::Presence {
                event: "joined",
                user: UserRef {
                    id: user_id,
                    username,
                },
            });
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
