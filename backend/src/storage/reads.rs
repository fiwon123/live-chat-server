//! Read-state persistence: the newest message each member has seen in a room.
//!
//! A cursor is a message id (or `NULL` when the member has never read the
//! room). Unread counts compare against `(created_at, id)` so ties on the
//! millisecond never reorder history. Marking read only ever moves the cursor
//! forward — an out-of-order call cannot un-read messages.

use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Per-room unread total for one member.
#[derive(Debug, Clone, FromRow)]
pub struct RoomUnread {
    pub room_id: Uuid,
    pub unread: i64,
}

/// Read-state persistence operations.
#[derive(Debug, Clone)]
pub struct Reads {
    pool: PgPool,
}

impl Reads {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Advance the member's read cursor to `message_id`, never backwards.
    ///
    /// The `WHERE NOT EXISTS` guard rejects the insert when the stored cursor
    /// already sits at or after the target, so a late/duplicate call is a
    /// no-op. A missing row (or a `NULL` cursor) always writes.
    pub async fn mark_read(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        message_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"INSERT INTO message_reads (room_id, user_id, last_read_message_id)
               SELECT $1, $2, $3
               WHERE NOT EXISTS (
                   SELECT 1
                   FROM message_reads mr
                   JOIN messages new ON new.id = $3
                   JOIN messages old ON old.id = mr.last_read_message_id
                   WHERE mr.room_id = $1
                     AND mr.user_id = $2
                     AND (new.created_at, new.id) <= (old.created_at, old.id)
               )
               ON CONFLICT (room_id, user_id) DO UPDATE
               SET last_read_message_id = EXCLUDED.last_read_message_id,
                   updated_at = now()"#,
            room_id,
            user_id,
            message_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Unread totals for every room `user_id` is a member of, keyed by room.
    ///
    /// Only member rooms appear; rooms with nothing new (or no membership) are
    /// simply absent, so callers default them to zero.
    pub async fn unread_counts(&self, user_id: Uuid) -> Result<Vec<RoomUnread>, sqlx::Error> {
        sqlx::query_as!(
            RoomUnread,
            r#"SELECT rm.room_id,
                      count(m.id) FILTER (
                          WHERE mr.last_read_message_id IS NULL
                             OR (m.created_at, m.id) > (c.created_at, c.id)
                      ) AS "unread!"
               FROM room_memberships rm
               LEFT JOIN messages m ON m.room_id = rm.room_id
               LEFT JOIN message_reads mr
                      ON mr.room_id = rm.room_id AND mr.user_id = rm.user_id
               LEFT JOIN messages c ON c.id = mr.last_read_message_id
               WHERE rm.user_id = $1
               GROUP BY rm.room_id"#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Unread total for a single room and member (`0` when unknown/no cursor).
    pub async fn unread_count(&self, room_id: Uuid, user_id: Uuid) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar!(
            r#"SELECT count(m.id) AS "count!"
               FROM messages m
               LEFT JOIN message_reads mr
                      ON mr.room_id = m.room_id AND mr.user_id = $2
               LEFT JOIN messages c ON c.id = mr.last_read_message_id
               WHERE m.room_id = $1
                 AND (
                     mr.last_read_message_id IS NULL
                     OR (m.created_at, m.id) > (c.created_at, c.id)
                 )"#,
            room_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await
    }
}
