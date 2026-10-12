//! Reaction persistence: one row per (message, user, emoji), toggled by a
//! user and aggregated per message for responses.

use std::collections::HashMap;

use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Maximum length (chars) of an emoji string. Real emoji are at most a few
/// code points (skin tone + ZWJ sequences); anything longer is not an emoji.
pub const MAX_EMOJI_LEN: usize = 16;

/// One persisted reaction row, as stored.
#[derive(Debug, Clone, FromRow)]
pub struct ReactionRow {
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub emoji: String,
}

/// Aggregated reactions for a single message and emoji.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionGroup {
    pub emoji: String,
    pub user_ids: Vec<Uuid>,
}

impl ReactionGroup {
    pub fn count(&self) -> i64 {
        self.user_ids.len() as i64
    }
}

/// Reaction persistence operations.
#[derive(Debug, Clone)]
pub struct Reactions {
    pool: PgPool,
}

impl Reactions {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a reaction, ignoring duplicates. Returns `true` when a new row
    /// was written, `false` when the user had already reacted with that emoji.
    pub async fn insert(
        &self,
        message_id: Uuid,
        user_id: Uuid,
        emoji: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            r#"INSERT INTO message_reactions (message_id, user_id, emoji)
               VALUES ($1, $2, $3)
               ON CONFLICT DO NOTHING"#,
            message_id,
            user_id,
            emoji
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Remove a reaction. Returns `true` when a row was actually deleted.
    pub async fn remove(
        &self,
        message_id: Uuid,
        user_id: Uuid,
        emoji: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            r#"DELETE FROM message_reactions
               WHERE message_id = $1 AND user_id = $2 AND emoji = $3"#,
            message_id,
            user_id,
            emoji
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Toggle a reaction: remove it if present, otherwise add it. Returns
    /// `true` when the reaction is now present (added), `false` when removed.
    pub async fn toggle(
        &self,
        message_id: Uuid,
        user_id: Uuid,
        emoji: &str,
    ) -> Result<bool, sqlx::Error> {
        if self.remove(message_id, user_id, emoji).await? {
            return Ok(false);
        }
        self.insert(message_id, user_id, emoji).await?;
        Ok(true)
    }

    /// All reaction rows for the given messages, oldest first.
    pub async fn list_for_messages(
        &self,
        message_ids: &[Uuid],
    ) -> Result<Vec<ReactionRow>, sqlx::Error> {
        if message_ids.is_empty() {
            return Ok(Vec::new());
        }
        sqlx::query_as!(
            ReactionRow,
            r#"SELECT message_id, user_id, emoji
               FROM message_reactions
               WHERE message_id = ANY($1)
               ORDER BY created_at, user_id, emoji"#,
            message_ids
        )
        .fetch_all(&self.pool)
        .await
    }
}

/// Group flat reaction rows by message and emoji, preserving first-seen order.
pub fn group_by_message(rows: Vec<ReactionRow>) -> HashMap<Uuid, Vec<ReactionGroup>> {
    let mut by_message: HashMap<Uuid, Vec<ReactionGroup>> = HashMap::new();
    for row in rows {
        let groups = by_message.entry(row.message_id).or_default();
        match groups.iter_mut().find(|group| group.emoji == row.emoji) {
            Some(group) => {
                if !group.user_ids.contains(&row.user_id) {
                    group.user_ids.push(row.user_id);
                }
            }
            None => groups.push(ReactionGroup {
                emoji: row.emoji,
                user_ids: vec![row.user_id],
            }),
        }
    }
    by_message
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(message_id: Uuid, user_id: Uuid, emoji: &str) -> ReactionRow {
        ReactionRow {
            message_id,
            user_id,
            emoji: emoji.to_string(),
        }
    }

    #[test]
    fn groups_rows_by_message_and_emoji() {
        let m = Uuid::new_v4();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let grouped = group_by_message(vec![row(m, a, "👍"), row(m, b, "👍"), row(m, a, "🎉")]);
        let groups = grouped.get(&m).unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].emoji, "👍");
        assert_eq!(groups[0].count(), 2);
        assert_eq!(groups[1].emoji, "🎉");
        assert_eq!(groups[1].count(), 1);
    }

    #[test]
    fn deduplicates_a_repeated_user() {
        let m = Uuid::new_v4();
        let a = Uuid::new_v4();
        let grouped = group_by_message(vec![row(m, a, "👍"), row(m, a, "👍")]);
        assert_eq!(grouped.get(&m).unwrap()[0].count(), 1);
    }

    #[test]
    fn separates_messages() {
        let m1 = Uuid::new_v4();
        let m2 = Uuid::new_v4();
        let a = Uuid::new_v4();
        let grouped = group_by_message(vec![row(m1, a, "👍"), row(m2, a, "👍")]);
        assert_eq!(grouped.len(), 2);
    }
}
