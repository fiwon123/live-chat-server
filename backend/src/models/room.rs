use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// A chat room.
#[derive(Debug, Clone, FromRow)]
pub struct Room {
    pub id: Uuid,
    pub name: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}
