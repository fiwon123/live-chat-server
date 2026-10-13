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

    /// Create a room and add the creator as its first member, but only if the
    /// creator currently owns fewer than `max_rooms` rooms.
    ///
    /// The whole thing runs in one transaction that first takes a per-user
    /// advisory lock, so a user's concurrent creates are serialized: the count
    /// below only ever sees creates that already committed, and the cap cannot
    /// be exceeded. (A bare single-statement `INSERT ... WHERE count(...) < n`
    /// is *not* enough — under READ COMMITTED two statements take snapshots
    /// that miss each other's uncommitted rows.)
    ///
    /// Both inserts share the transaction, so a room row can never exist
    /// without its creator's membership. Returns `Ok(None)` at the cap.
    ///
    /// Runtime `query_as` (not the `query_as!` macro) is deliberate: the SQL
    /// text is built here at runtime, so it does not need a `.sqlx` entry.
    pub async fn create_within_limit(
        &self,
        name: &str,
        created_by: Uuid,
        max_rooms: i64,
    ) -> Result<Option<Room>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // Serialize this user's creates for the life of the transaction. The
        // key is a hash of the user id; a collision only costs some extra
        // serialization between two users, never correctness.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 0))")
            .bind(created_by.to_string())
            .execute(&mut *tx)
            .await?;

        let room = sqlx::query_as::<_, Room>(
            r#"WITH new_room AS (
                   INSERT INTO rooms (name, created_by)
                   SELECT $1, $2
                   WHERE (
                       SELECT count(*) FROM rooms WHERE created_by = $2
                   ) < $3
                   RETURNING id, name, created_by, created_at
               ), join_creator AS (
                   INSERT INTO room_memberships (room_id, user_id)
                   SELECT id, $2 FROM new_room
               )
               SELECT id, name, created_by, created_at FROM new_room"#,
        )
        .bind(name)
        .bind(created_by)
        .bind(max_rooms)
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(room)
    }

    /// Delete a room by id. Memberships, messages, reactions, and read cursors
    /// cascade via the schema's `ON DELETE CASCADE` constraints. Returns
    /// `false` when no room with that id existed.
    pub async fn delete(&self, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM rooms WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
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
