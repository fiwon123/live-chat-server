use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::models::Message;

/// A message joined with its author's username, for history responses.
#[derive(Debug, Clone, FromRow)]
pub struct MessageWithAuthor {
    pub id: Uuid,
    pub room_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub body: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Message persistence operations.
#[derive(Debug, Clone)]
pub struct Messages {
    pool: PgPool,
}

impl Messages {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Persist a message. Called BEFORE broadcasting so receivers see a row
    /// with a stable id/timestamp; on error nothing is broadcast.
    pub async fn insert(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        body: &str,
    ) -> Result<Message, sqlx::Error> {
        sqlx::query_as!(
            Message,
            r#"INSERT INTO messages (room_id, user_id, body)
               VALUES ($1, $2, $3)
               RETURNING id, room_id, user_id, body, created_at"#,
            room_id,
            user_id,
            body
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Newest-first history, capped at `limit` (1..=100), optionally before a
    /// cursor message — used for pagination and reconnect snapshots.
    pub async fn list(
        &self,
        room_id: Uuid,
        before: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<Message>, sqlx::Error> {
        let limit = limit.clamp(1, 100);
        match before {
            Some(cursor) => {
                sqlx::query_as!(
                    Message,
                    r#"SELECT m.id, m.room_id, m.user_id, m.body, m.created_at
                       FROM messages m
                       JOIN messages c ON c.id = $2 AND c.room_id = $1
                       WHERE m.room_id = $1
                         AND (m.created_at, m.id) < (c.created_at, c.id)
                       ORDER BY m.created_at DESC, m.id DESC
                       LIMIT $3"#,
                    room_id,
                    cursor,
                    limit
                )
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as!(
                    Message,
                    r#"SELECT id, room_id, user_id, body, created_at
                       FROM messages
                       WHERE room_id = $1
                       ORDER BY created_at DESC, id DESC
                       LIMIT $2"#,
                    room_id,
                    limit
                )
                .fetch_all(&self.pool)
                .await
            }
        }
    }

    /// Newest-first history with the author's username, capped at `limit`
    /// (1..=100), optionally before a cursor message.
    pub async fn list_with_author(
        &self,
        room_id: Uuid,
        before: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<MessageWithAuthor>, sqlx::Error> {
        let limit = limit.clamp(1, 100);
        match before {
            Some(cursor) => {
                sqlx::query_as!(
                    MessageWithAuthor,
                    r#"SELECT m.id, m.room_id, m.user_id, u.username, m.body, m.created_at
                       FROM messages m
                       JOIN users u ON u.id = m.user_id
                       JOIN messages c ON c.id = $2 AND c.room_id = $1
                       WHERE m.room_id = $1
                         AND (m.created_at, m.id) < (c.created_at, c.id)
                       ORDER BY m.created_at DESC, m.id DESC
                       LIMIT $3"#,
                    room_id,
                    cursor,
                    limit
                )
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as!(
                    MessageWithAuthor,
                    r#"SELECT m.id, m.room_id, m.user_id, u.username, m.body, m.created_at
                       FROM messages m
                       JOIN users u ON u.id = m.user_id
                       WHERE m.room_id = $1
                       ORDER BY m.created_at DESC, m.id DESC
                       LIMIT $2"#,
                    room_id,
                    limit
                )
                .fetch_all(&self.pool)
                .await
            }
        }
    }

    /// The newest message in a room with its author, for room-list previews.
    pub async fn latest_with_author(
        &self,
        room_id: Uuid,
    ) -> Result<Option<MessageWithAuthor>, sqlx::Error> {
        sqlx::query_as!(
            MessageWithAuthor,
            r#"SELECT m.id, m.room_id, m.user_id, u.username, m.body, m.created_at
               FROM messages m
               JOIN users u ON u.id = m.user_id
               WHERE m.room_id = $1
               ORDER BY m.created_at DESC, m.id DESC
               LIMIT 1"#,
            room_id
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// The newest message of every room, one row per room (for `GET /api/rooms`).
    pub async fn latest_with_author_by_room(&self) -> Result<Vec<MessageWithAuthor>, sqlx::Error> {
        sqlx::query_as!(
            MessageWithAuthor,
            r#"SELECT DISTINCT ON (m.room_id)
                      m.id, m.room_id, m.user_id, u.username, m.body, m.created_at
               FROM messages m
               JOIN users u ON u.id = m.user_id
               ORDER BY m.room_id, m.created_at DESC, m.id DESC"#
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Find a single message by id (used for cursor validation).
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Message>, sqlx::Error> {
        sqlx::query_as!(
            Message,
            r#"SELECT id, room_id, user_id, body, created_at
               FROM messages
               WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
    }
}
