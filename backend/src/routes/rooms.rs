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
use crate::services::RoomService;
use crate::storage::MessageWithAuthor;

/// Public representation of a room.
#[derive(Debug, Serialize)]
pub struct RoomResponse {
    pub id: Uuid,
    pub name: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub member: bool,
}

impl RoomResponse {
    fn from_storage(room: crate::storage::RoomWithMembership) -> Self {
        Self {
            id: room.id,
            name: room.name,
            created_by: room.created_by,
            created_at: room.created_at,
            member: room.member,
        }
    }
}

/// Request body for creating a room.
#[derive(Debug, Deserialize)]
pub struct CreateRoomRequest {
    pub name: String,
}

/// Public representation of a message with its author's username.
#[derive(Debug, Clone, Serialize)]
pub struct MessageResponse {
    pub id: Uuid,
    pub room_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
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
    let response: Vec<RoomResponse> = rooms.into_iter().map(RoomResponse::from_storage).collect();
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
    Ok(Json(RoomResponse::from_storage(room)))
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
    let response: Vec<MessageResponse> = messages.into_iter().map(MessageResponse::from).collect();
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
        .route("/rooms/{id}/messages", get(room_history))
}
