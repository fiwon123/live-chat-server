//! Room and message-history HTTP endpoints.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::services::{RoomService, RoomSummary};
use crate::storage::{MessageWithAuthor, Users};
use crate::ws::{UserRef, WsOutgoing};

/// Public representation of a room.
#[derive(Debug, Serialize)]
pub struct RoomResponse {
    pub id: Uuid,
    pub name: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub member: bool,
    /// Messages newer than the caller's read cursor; always `0` on create.
    pub unread_count: i64,
    /// Newest message in the room, for list previews; `null` when empty.
    pub last_message: Option<LastMessageResponse>,
}

/// Compact latest-message preview attached to a room.
#[derive(Debug, Clone, Serialize)]
pub struct LastMessageResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

impl From<MessageWithAuthor> for LastMessageResponse {
    fn from(message: MessageWithAuthor) -> Self {
        Self {
            id: message.id,
            user_id: message.user_id,
            username: message.username,
            body: message.body,
            created_at: message.created_at,
        }
    }
}

impl RoomResponse {
    fn from_summary(summary: RoomSummary) -> Self {
        Self {
            id: summary.room.id,
            name: summary.room.name,
            created_by: summary.room.created_by,
            created_at: summary.room.created_at,
            member: summary.room.member,
            unread_count: summary.unread_count,
            last_message: summary.last_message.map(LastMessageResponse::from),
        }
    }
}

/// Request body for creating a room.
#[derive(Debug, Deserialize)]
pub struct CreateRoomRequest {
    pub name: String,
}

/// Public representation of a message with its author's username.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub id: Uuid,
    pub room_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    /// Aggregated reactions, oldest emoji first. Empty for a fresh message.
    pub reactions: Vec<ReactionResponse>,
}

/// Public representation of one emoji's reactions on a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionResponse {
    pub emoji: String,
    pub count: i64,
    pub user_ids: Vec<Uuid>,
}

impl From<crate::storage::ReactionGroup> for ReactionResponse {
    fn from(group: crate::storage::ReactionGroup) -> Self {
        Self {
            count: group.count(),
            emoji: group.emoji,
            user_ids: group.user_ids,
        }
    }
}

impl From<MessageWithAuthor> for MessageResponse {
    fn from(message: MessageWithAuthor) -> Self {
        Self {
            id: message.id,
            room_id: message.room_id,
            user_id: message.user_id,
            username: message.username,
            body: message.body,
            created_at: message.created_at,
            reactions: Vec::new(),
        }
    }
}

/// Query parameters for the history endpoint.
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    /// Page size (1-100, default 50, clamped server-side).
    pub limit: Option<i64>,
    /// Message-id cursor: return messages strictly older than this one.
    pub before: Option<Uuid>,
}

/// `POST /api/rooms`
pub async fn create_room(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<CreateRoomRequest>,
) -> Result<(StatusCode, Json<RoomResponse>), AppError> {
    let service = room_service(&state);
    let room = service.create(user.id, &request.name).await?;
    let response = RoomResponse {
        id: room.id,
        name: room.name,
        created_by: room.created_by,
        created_at: room.created_at,
        member: true,
        unread_count: 0,
        last_message: None,
    };
    Ok((StatusCode::CREATED, Json(response)))
}

/// `GET /api/rooms`
pub async fn list_rooms(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<RoomResponse>>, AppError> {
    let service = room_service(&state);
    let rooms = service.list(user.id).await?;
    let response: Vec<RoomResponse> = rooms.into_iter().map(RoomResponse::from_summary).collect();
    Ok(Json(response))
}

/// `GET /api/rooms/{id}`
pub async fn get_room(
    State(state): State<AppState>,
    user: AuthUser,
    Path(room_id): Path<Uuid>,
) -> Result<Json<RoomResponse>, AppError> {
    let service = room_service(&state);
    let room = service.get(room_id, user.id).await?;
    Ok(Json(RoomResponse::from_summary(room)))
}

/// `POST /api/rooms/{id}/join`
pub async fn join_room(
    State(state): State<AppState>,
    user: AuthUser,
    Path(room_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let service = room_service(&state);
    service.join(room_id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/rooms/{id}/leave`
pub async fn leave_room(
    State(state): State<AppState>,
    user: AuthUser,
    Path(room_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let service = room_service(&state);
    service.leave(room_id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Request body for `POST /api/rooms/{id}/read` — a specific message to mark
/// read, or omitted/`null` to mark everything read.
#[derive(Debug, Default, Deserialize)]
pub struct MarkReadRequest {
    #[serde(default)]
    pub message_id: Option<Uuid>,
}

/// Response for marking a room read: the effective cursor and remaining unread.
#[derive(Debug, Serialize)]
pub struct ReadResponse {
    pub room_id: Uuid,
    pub last_read_message_id: Option<Uuid>,
    pub unread_count: i64,
}

/// `POST /api/rooms/{id}/read`
///
/// Persists the caller's read cursor (member-only) and broadcasts an ephemeral
/// `read` receipt to the room so others can show who has seen the message. The
/// persisted cursor is the source of truth; the broadcast is best-effort.
pub async fn mark_room_read(
    State(state): State<AppState>,
    user: AuthUser,
    Path(room_id): Path<Uuid>,
    Json(request): Json<MarkReadRequest>,
) -> Result<Json<ReadResponse>, AppError> {
    let user_record = Users::new(state.pool.clone())
        .find_by_id(user.id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("user no longer exists".to_string()))?;

    let service = room_service(&state);
    let result = service
        .mark_read(room_id, user.id, request.message_id)
        .await?;

    if let Some(message_id) = result.message_id {
        state
            .registry
            .send(
                room_id,
                WsOutgoing::Read {
                    message_id,
                    user: UserRef {
                        id: user.id,
                        username: user_record.username,
                    },
                },
            )
            .await;
    }

    Ok(Json(ReadResponse {
        room_id,
        last_read_message_id: result.message_id,
        unread_count: result.unread_count,
    }))
}

/// `GET /api/rooms/{id}/messages`
pub async fn room_history(
    State(state): State<AppState>,
    user: AuthUser,
    Path(room_id): Path<Uuid>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<MessageResponse>>, AppError> {
    let service = room_service(&state);
    let messages = service
        .history(room_id, user.id, query.before, query.limit.unwrap_or(50))
        .await?;

    // One extra query for every reaction on this page, then attach.
    let ids: Vec<Uuid> = messages.iter().map(|message| message.id).collect();
    let mut reactions = service.reactions_for_messages(&ids).await?;
    let response: Vec<MessageResponse> = messages
        .into_iter()
        .map(|message| {
            let mut response = MessageResponse::from(message);
            if let Some(groups) = reactions.remove(&response.id) {
                response.reactions = groups.into_iter().map(ReactionResponse::from).collect();
            }
            response
        })
        .collect();
    Ok(Json(response))
}

fn room_service(state: &AppState) -> RoomService {
    RoomService::new(state.pool.clone())
}

/// Routes nested under `/api`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/rooms", get(list_rooms).post(create_room))
        .route("/rooms/{id}", get(get_room))
        .route("/rooms/{id}/join", post(join_room))
        .route("/rooms/{id}/leave", post(leave_room))
        .route("/rooms/{id}/read", post(mark_room_read))
        .route("/rooms/{id}/messages", get(room_history))
}
