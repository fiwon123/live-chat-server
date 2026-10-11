use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::models::Room;

/// Room persistence operations.
#[derive(Debug, Clone)]
pub struct Rooms {
    pool: PgPool,
}

impl Rooms {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a room; the caller adds the creator as a member.
    pub async fn create(&self, name: &str, created_by: Uuid) -> Result<Room, sqlx::Error> {
        sqlx::query_as!(
            Room,
            r#"INSERT INTO rooms (name, created_by)
               VALUES ($1, $2)
               RETURNING id, name, created_by, created_at"#,
            name,
            created_by
        )
        .fetch_one(&self.pool)
        .await
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Room>, sqlx::Error> {
        sqlx::query_as!(
            Room,
            r#"SELECT id, name, created_by, created_at
               FROM rooms
               WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// A room plus whether the caller is a member (for list UI "joined" flag).
    pub async fn list_with_membership(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<RoomWithMembership>, sqlx::Error> {
        sqlx::query_as!(
            RoomWithMembership,
            r#"SELECT r.id, r.name, r.created_by, r.created_at,
                      EXISTS (
                          SELECT 1 FROM room_memberships m
                          WHERE m.room_id = r.id AND m.user_id = $1
                      ) AS "member!"
               FROM rooms r
               ORDER BY r.created_at, r.id"#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
    }
}

/// A room paired with the viewer's membership status.
#[derive(Debug, Clone, FromRow)]
pub struct RoomWithMembership {
    pub id: Uuid,
    pub name: String,
    pub created_by: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub member: bool,
}
