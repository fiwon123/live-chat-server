//! Auth extractor: resolves the current user from a bearer token.

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use uuid::Uuid;

use crate::AppState;
use crate::auth::jwt;
use crate::error::AppError;

/// The authenticated user for a request, derived from `Authorization: Bearer`.
///
/// Carries only the user id from the verified token; handlers that need the
/// full record (e.g. `GET /api/me`) load it from storage.
#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub id: Uuid,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .ok_or_else(|| AppError::Unauthorized("missing bearer token".to_string()))?;

        let id = jwt::verify_token(&state.config.jwt_secret, token)?;
        Ok(AuthUser { id })
    }
}
