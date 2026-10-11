//! Application error type mapped to HTTP responses.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// Errors surfaced by handlers.
///
/// Every variant maps to a status code; the client always receives
/// `{"error": "<message>"}`. Internal details are logged, never returned.
#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    Internal(anyhow::Error),
}

impl AppError {
    /// Status code and client-safe message for this error.
    fn parts(&self) -> (StatusCode, String) {
        match self {
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.clone()),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, m.clone()),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            AppError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error".to_string(),
            ),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if let AppError::Internal(ref err) = self {
            tracing::error!(error = ?err, "request failed with an internal error");
        }
        let (status, message) = self.parts();
        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (status, message) = self.parts();
        write!(f, "{status}: {message}")
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Internal(err.into())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err)
    }
}

impl AppError {
    /// True when the database error is a unique-constraint violation.
    pub fn is_unique_violation(err: &sqlx::Error) -> bool {
        err.as_database_error()
            .and_then(|db| db.code())
            .is_some_and(|code| code == "23505")
    }
}
