use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// A registered chat user.
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}
