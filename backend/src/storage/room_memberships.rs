use sqlx::PgPool;
use uuid::Uuid;

/// Room-membership persistence operations.
#[derive(Debug, Clone)]
pub struct RoomMemberships {
    pool: PgPool,
}

impl RoomMemberships {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Add a user to a room (idempotent).
    pub async fn join(&self, room_id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"INSERT INTO room_memberships (room_id, user_id)
               VALUES ($1, $2)
               ON CONFLICT (room_id, user_id) DO NOTHING"#,
            room_id,
            user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Remove a user from a room (idempotent). Returns the number of rows
    /// deleted so callers can tell whether the user was actually a member.
    pub async fn leave(&self, room_id: Uuid, user_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            r#"DELETE FROM room_memberships
               WHERE room_id = $1 AND user_id = $2"#,
            room_id,
            user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn is_member(&self, room_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let row = sqlx::query!(
            r#"SELECT EXISTS (
                   SELECT 1 FROM room_memberships
                   WHERE room_id = $1 AND user_id = $2
               ) AS "member!"
            "#,
            room_id,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.member)
    }
}
