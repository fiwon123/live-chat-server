use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// A room membership: which users may read/write a room.
#[derive(Debug, Clone, FromRow)]
pub struct RoomMembership {
    pub room_id: Uuid,
    pub user_id: Uuid,
    pub joined_at: DateTime<Utc>,
}
