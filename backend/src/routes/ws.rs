//! WebSocket upgrade endpoint.

use axum::Router;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Query, State};
use axum::response::Response;
use axum::routing::get;
use serde::Deserialize;
use uuid::Uuid;

use crate::AppState;
use crate::auth::jwt;
use crate::error::AppError;
use crate::storage::{RoomMemberships, Rooms, Users};
use crate::ws::run_session;

/// Query parameters for the WebSocket handshake.
///
/// Browsers cannot set an `Authorization` header on WebSocket connections, so
/// the JWT travels as a query parameter. Keep it out of logs where possible.
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub token: String,
    pub room_id: Uuid,
}

/// `GET /api/ws?token=...&room_id=...`
///
/// Verifies the token, then the room's existence and the user's membership
/// BEFORE upgrading; failures reject the handshake with a JSON error.
pub async fn ws_handler(
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Result<Response, AppError> {
    let user_id = jwt::verify_token(&state.config.jwt_secret, &query.token)?;

    let user = Users::new(state.pool.clone())
        .find_by_id(user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("user no longer exists".to_string()))?;

    if Rooms::new(state.pool.clone())
        .find_by_id(query.room_id)
        .await?
        .is_none()
    {
        return Err(AppError::NotFound("room not found".to_string()));
    }
    if !RoomMemberships::new(state.pool.clone())
        .is_member(query.room_id, user_id)
        .await?
    {
        return Err(AppError::Forbidden(
            "you are not a member of this room".to_string(),
        ));
    }

    let registry = state.registry.clone();
    let pool = state.pool.clone();
    let room_id = query.room_id;
    let username = user.username;

    tracing::info!(room_id = %room_id, user_id = %user_id, "ws connected");
    // Bound per-message allocation before JSON parsing; bodies are capped at
    // MAX_MESSAGE_LEN (4 KB) but an oversized frame would otherwise allocate
    // up to axum's 64 MiB default before the length check could reject it.
    Ok(ws
        .max_message_size(64 * 1024)
        .on_upgrade(move |socket| run_session(socket, registry, pool, room_id, user_id, username)))
}

/// Routes nested under `/api`.
pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(ws_handler))
}
