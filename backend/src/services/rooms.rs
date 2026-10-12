//! Room lifecycle: create, list, get, join, leave, and history access.

use std::collections::HashMap;

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::Room;
use crate::storage::{
    MessageWithAuthor, Messages, ReactionGroup, Reactions, RoomMemberships, RoomWithMembership,
    Rooms, group_by_message,
};

/// Maximum length of a room name (after trimming).
pub const MAX_ROOM_NAME_LEN: usize = 64;

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
}

impl RoomService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            rooms: Rooms::new(pool.clone()),
            memberships: RoomMemberships::new(pool.clone()),
            messages: Messages::new(pool.clone()),
            reactions: Reactions::new(pool),
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

    /// All rooms, annotated with the caller's membership.
    pub async fn list(&self, user_id: Uuid) -> Result<Vec<RoomWithMembership>, AppError> {
        Ok(self.rooms.list_with_membership(user_id).await?)
    }

    /// A single room plus the caller's membership; `404` when unknown.
    pub async fn get(&self, room_id: Uuid, user_id: Uuid) -> Result<RoomWithMembership, AppError> {
        let room = self
            .rooms
            .find_by_id(room_id)
            .await?
            .ok_or_else(|| AppError::NotFound("room not found".to_string()))?;
        let member = self.memberships.is_member(room_id, user_id).await?;
        Ok(RoomWithMembership {
            id: room.id,
            name: room.name,
            created_by: room.created_by,
            created_at: room.created_at,
            member,
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
