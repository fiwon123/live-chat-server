//! Optional Redis Pub/Sub fan-out for multi-instance broadcasting.
//!
//! When a second server instance exists, room events are published to a
//! dedicated `room:{uuid}` channel and each connected session subscribes to
//! its room's channel. The payload is the exact JSON a client would receive,
//! so instances exchange the same event stream they would have produced
//! locally.
//!
//! Without `REDIS_URL` the registry stays fully in-memory (`RoomRegistry::new`)
//! and this module is never exercised.

use std::fmt;

use futures_util::StreamExt;
use redis::aio::{ConnectionManager, PubSub, PubSubStream};
use redis::{AsyncCommands, Client};
use uuid::Uuid;

use crate::ws::WsOutgoing;

/// Redis key a room's events are published on.
fn channel(room_id: Uuid) -> String {
    format!("room:{room_id}")
}

/// A failure when publishing or receiving a room event over Redis.
#[derive(Debug)]
pub enum RedisPubSubError {
    /// Redis connection/command failure.
    Redis(redis::RedisError),
    /// The event payload could not be (de)serialized.
    Json(serde_json::Error),
}

impl fmt::Display for RedisPubSubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Redis(err) => write!(f, "redis: {err}"),
            Self::Json(err) => write!(f, "event (de)serialization: {err}"),
        }
    }
}

impl std::error::Error for RedisPubSubError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Redis(err) => Some(err),
            Self::Json(err) => Some(err),
        }
    }
}

impl From<redis::RedisError> for RedisPubSubError {
    fn from(err: redis::RedisError) -> Self {
        Self::Redis(err)
    }
}

impl From<serde_json::Error> for RedisPubSubError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

/// Cloneable handle to a Redis connection used for publishing; subscribers are
/// per-session streams, so each subscription gets its own connection.
#[derive(Clone)]
pub struct RedisPubSub {
    client: Client,
    /// Multiplexed, auto-reconnecting connection for `publish` calls.
    conn: ConnectionManager,
}

impl RedisPubSub {
    /// Connect and verify the server is reachable. Callers treat a failure as
    /// "stay in-memory" and log a warning.
    pub async fn connect(url: &str) -> redis::RedisResult<Self> {
        let client = Client::open(url)?;
        let conn = ConnectionManager::new(client.clone()).await?;
        Ok(Self { client, conn })
    }

    /// Fan an event out to every instance with subscribers in the room.
    pub async fn publish(&self, room_id: Uuid, event: WsOutgoing) -> Result<(), RedisPubSubError> {
        let payload = serde_json::to_string(&event)?;
        // `AsyncCommands` needs a mutable handle; cloning a ConnectionManager
        // is cheap (it is a shared, multiplexed connection).
        let mut conn = self.conn.clone();
        // Annotate the receiver count so inference pins `RV = usize`.
        let _receivers: usize = conn.publish(channel(room_id), payload).await?;
        Ok(())
    }

    /// Open a dedicated subscription for one room (one Redis connection per
    /// session in pub/sub mode).
    pub async fn subscribe(&self, room_id: Uuid) -> redis::RedisResult<RoomSubscriber> {
        let mut pubsub: PubSub = self.client.get_async_pubsub().await?;
        pubsub.subscribe(channel(room_id)).await?;
        Ok(RoomSubscriber {
            stream: pubsub.into_on_message(),
        })
    }
}

/// One session's subscription to a room's event stream.
pub struct RoomSubscriber {
    stream: PubSubStream,
}

impl RoomSubscriber {
    /// Yield the next event, or `None` when the subscription ended (the Redis
    /// connection dropped — the caller closes the session; the client
    /// reconnects).
    pub async fn next(&mut self) -> Option<Result<WsOutgoing, RedisPubSubError>> {
        self.stream.next().await.map(|msg| {
            let payload: String = msg.get_payload()?;
            Ok(serde_json::from_str(&payload)?)
        })
    }
}
