use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// A single chat message.
#[derive(Debug, Clone, FromRow)]
pub struct Message {
    pub id: Uuid,
    pub room_id: Uuid,
    pub user_id: Uuid,
    pub body: String,
    pub created_at: DateTime<Utc>,
}
