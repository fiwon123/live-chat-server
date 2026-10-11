//! Auth HTTP endpoints.

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::User;
use crate::services::AuthService;
use crate::storage::Users;

/// Credentials accepted by register and login.
#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

/// Public representation of a user (never includes the password hash).
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            created_at: user.created_at,
        }
    }
}

/// Successful auth response: the access token and its user.
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

/// `POST /api/auth/register`
pub async fn register(
    State(state): State<AppState>,
    Json(credentials): Json<Credentials>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    let service = auth_service(&state);
    let authenticated = service
        .register(&credentials.username, &credentials.password)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token: authenticated.token,
            user: authenticated.user.into(),
        }),
    ))
}

/// `POST /api/auth/login`
pub async fn login(
    State(state): State<AppState>,
    Json(credentials): Json<Credentials>,
) -> Result<Json<AuthResponse>, AppError> {
    let service = auth_service(&state);
    let authenticated = service
        .login(&credentials.username, &credentials.password)
        .await?;

    Ok(Json(AuthResponse {
        token: authenticated.token,
        user: authenticated.user.into(),
    }))
}

/// `GET /api/me`
pub async fn me(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<UserResponse>, AppError> {
    let users = Users::new(state.pool.clone());
    let user = users
        .find_by_id(user.id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("user no longer exists".to_string()))?;
    Ok(Json(user.into()))
}

fn auth_service(state: &AppState) -> AuthService {
    AuthService::new(
        Users::new(state.pool.clone()),
        state.config.jwt_secret.clone(),
    )
}

/// Routes nested under `/api`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/me", axum::routing::get(me))
}
