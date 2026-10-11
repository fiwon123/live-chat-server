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
            .and_then(parse_bearer)
            .ok_or_else(|| AppError::Unauthorized("missing bearer token".to_string()))?;

        let id = jwt::verify_token(&state.config.jwt_secret, token)?;
        Ok(AuthUser { id })
    }
}

/// Extract the credentials from an `Authorization` header value.
///
/// The scheme is compared case-insensitively per RFC 7235, so `bearer`, `Bearer`,
/// and `BEARER` are all accepted.
fn parse_bearer(header: &str) -> Option<&str> {
    let (scheme, token) = header.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = token.trim();
    (!token.is_empty()).then_some(token)
}

#[cfg(test)]
mod tests {
    use super::parse_bearer;

    #[test]
    fn accepts_any_case_scheme() {
        assert_eq!(parse_bearer("Bearer abc.def.ghi"), Some("abc.def.ghi"));
        assert_eq!(parse_bearer("bearer abc.def.ghi"), Some("abc.def.ghi"));
        assert_eq!(parse_bearer("BEARER abc.def.ghi"), Some("abc.def.ghi"));
    }

    #[test]
    fn rejects_missing_or_malformed() {
        assert_eq!(parse_bearer("Basic abc"), None);
        assert_eq!(parse_bearer("Bearertoken"), None);
        assert_eq!(parse_bearer("Bearer "), None);
        assert_eq!(parse_bearer(""), None);
    }
}
