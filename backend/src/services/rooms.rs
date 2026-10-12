//! Room lifecycle: create, list, get, join, leave, and history access.

use std::collections::HashMap;

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::Room;
use crate::storage::{
    MessageWithAuthor, Messages, ReactionGroup, Reactions, Reads, RoomMemberships,
    RoomWithMembership, Rooms, group_by_message,
};

/// Maximum length of a room name (after trimming).
pub const MAX_ROOM_NAME_LEN: usize = 64;

/// A room plus the caller-specific read state shown in room lists.
#[derive(Debug, Clone)]
pub struct RoomSummary {
    pub room: RoomWithMembership,
    pub unread_count: i64,
    pub last_message: Option<MessageWithAuthor>,
}

/// Outcome of marking a room read: the effective cursor and the remaining
/// unread total (zero when the newest message was marked).
#[derive(Debug, Clone, Copy)]
pub struct MarkReadResult {
    pub message_id: Option<Uuid>,
    pub unread_count: i64,
}

/// Validate a room name: trimmed, non-empty, at most 64 chars, no control
/// characters (embedded NULs must not reach Postgres).
fn validate_room_name(name: &str) -> Result<String, AppError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest(
            "room name must not be empty".to_string(),
        ));
    }
    if name.chars().count() > MAX_ROOM_NAME_LEN {
        return Err(AppError::BadRequest(format!(
            "room name must be at most {MAX_ROOM_NAME_LEN} characters"
        )));
    }
    if name.chars().any(char::is_control) {
        return Err(AppError::BadRequest(
            "room name must not contain control characters".to_string(),
        ));
    }
    Ok(name)
}

/// Room use-cases over the storage layer.
#[derive(Clone)]
pub struct RoomService {
    rooms: Rooms,
    memberships: RoomMemberships,
    messages: Messages,
    reactions: Reactions,
    reads: Reads,
}

impl RoomService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            rooms: Rooms::new(pool.clone()),
            memberships: RoomMemberships::new(pool.clone()),
            messages: Messages::new(pool.clone()),
            reactions: Reactions::new(pool.clone()),
            reads: Reads::new(pool),
        }
    }

    /// Create a room; the creator becomes its first member (atomic in
    /// storage — a room row can never exist without the creator's membership).
    pub async fn create(&self, user_id: Uuid, name: &str) -> Result<Room, AppError> {
        let name = validate_room_name(name)?;
        let room = self.rooms.create(&name, user_id).await?;
        tracing::info!(room_id = %room.id, user_id = %user_id, "room created");
        Ok(room)
    }

    /// All rooms, annotated with the caller's membership status, unread total,
    /// and latest-message preview.
    pub async fn list(&self, user_id: Uuid) -> Result<Vec<RoomSummary>, AppError> {
        let rooms = self.rooms.list_with_membership(user_id).await?;
        let unread = self.reads.unread_counts(user_id).await?;
        let mut unread: HashMap<Uuid, i64> = unread
            .into_iter()
            .map(|row| (row.room_id, row.unread))
            .collect();
        let mut latest: HashMap<Uuid, MessageWithAuthor> = self
            .messages
            .latest_with_author_by_room()
            .await?
            .into_iter()
            .map(|message| (message.room_id, message))
            .collect();

        Ok(rooms
            .into_iter()
            .map(|room| RoomSummary {
                unread_count: unread.remove(&room.id).unwrap_or(0),
                last_message: latest.remove(&room.id),
                room,
            })
            .collect())
    }

    /// A single room plus the caller's membership, unread total, and latest
    /// message; `404` when unknown. No membership is required to view a room.
    pub async fn get(&self, room_id: Uuid, user_id: Uuid) -> Result<RoomSummary, AppError> {
        let room = self
            .rooms
            .find_by_id(room_id)
            .await?
            .ok_or_else(|| AppError::NotFound("room not found".to_string()))?;
        let member = self.memberships.is_member(room_id, user_id).await?;
        let unread_count = self.reads.unread_count(room_id, user_id).await?;
        let last_message = self.messages.latest_with_author(room_id).await?;
        Ok(RoomSummary {
            room: RoomWithMembership {
                id: room.id,
                name: room.name,
                created_by: room.created_by,
                created_at: room.created_at,
                member,
            },
            unread_count,
            last_message,
        })
    }

    /// Mark a room read for a member: `message_id` advances the cursor to a
    /// specific message; `None` marks everything read (the newest message).
    ///
    /// `403` for non-members, `404` for unknown rooms, `400` when the target
    /// message is not in this room. An empty room persists nothing.
    pub async fn mark_read(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        message_id: Option<Uuid>,
    ) -> Result<MarkReadResult, AppError> {
        self.ensure_room_exists(room_id).await?;
        if !self.memberships.is_member(room_id, user_id).await? {
            return Err(AppError::Forbidden(
                "you are not a member of this room".to_string(),
            ));
        }

        let target = match message_id {
            Some(id) => match self.messages.find_by_id(id).await? {
                Some(message) if message.room_id == room_id => Some(message.id),
                _ => {
                    return Err(AppError::BadRequest(
                        "message not found in this room".to_string(),
                    ));
                }
            },
            None => self
                .messages
                .list(room_id, None, 1)
                .await?
                .into_iter()
                .next()
                .map(|message| message.id),
        };

        if let Some(id) = target {
            self.reads.mark_read(room_id, user_id, id).await?;
        }
        let unread_count = self.reads.unread_count(room_id, user_id).await?;
        Ok(MarkReadResult {
            message_id: target,
            unread_count,
        })
    }

    /// Join a room (idempotent); `404` when the room does not exist.
    pub async fn join(&self, room_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        self.ensure_room_exists(room_id).await?;
        self.memberships.join(room_id, user_id).await?;
        tracing::info!(room_id = %room_id, user_id = %user_id, "user joined room");
        Ok(())
    }

    /// Leave a room (idempotent); `404` when the room does not exist.
    pub async fn leave(&self, room_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        self.ensure_room_exists(room_id).await?;
        self.memberships.leave(room_id, user_id).await?;
        tracing::info!(room_id = %room_id, user_id = %user_id, "user left room");
        Ok(())
    }

    /// Message history for a member; `403` for non-members, `404` for unknown rooms.
    pub async fn history(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        before: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<MessageWithAuthor>, AppError> {
        self.ensure_room_exists(room_id).await?;
        if !self.memberships.is_member(room_id, user_id).await? {
            return Err(AppError::Forbidden(
                "you are not a member of this room".to_string(),
            ));
        }
        Ok(self
            .messages
            .list_with_author(room_id, before, limit)
            .await?)
    }

    /// Aggregated reactions for a page of messages, keyed by message id.
    /// Messages without reactions are absent from the map.
    pub async fn reactions_for_messages(
        &self,
        message_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<ReactionGroup>>, AppError> {
        Ok(group_by_message(
            self.reactions.list_for_messages(message_ids).await?,
        ))
    }

    async fn ensure_room_exists(&self, room_id: Uuid) -> Result<(), AppError> {
        if self.rooms.find_by_id(room_id).await?.is_none() {
            return Err(AppError::NotFound("room not found".to_string()));
        }
        Ok(())
    }
}
