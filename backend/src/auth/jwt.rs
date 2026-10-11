//! Stateless JWT creation and verification (HS256).

use anyhow::anyhow;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

/// Token lifetime handed out by the auth endpoints.
pub const TOKEN_TTL: Duration = Duration::hours(24);

/// JWT claims carried by an access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject: the user id as a UUID string.
    pub sub: String,
    /// Issued-at (unix seconds).
    pub iat: i64,
    /// Expiry (unix seconds).
    pub exp: i64,
}

/// Sign a token for `user_id`, expiring `ttl` from now.
pub fn create_token(secret: &str, user_id: Uuid, ttl: Duration) -> Result<String, AppError> {
    let now: DateTime<Utc> = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now.timestamp(),
        exp: (now + ttl).timestamp(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|err| AppError::Internal(anyhow!("failed to sign token: {err}")))
}

/// Verify a token and return its subject as a user id.
pub fn verify_token(secret: &str, token: &str) -> Result<Uuid, AppError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::Unauthorized("invalid or expired token".to_string()))?;

    Uuid::parse_str(&data.claims.sub)
        .map_err(|_| AppError::Unauthorized("invalid token subject".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-secret";

    #[test]
    fn round_trips_subject() {
        let id = Uuid::new_v4();
        let token = create_token(SECRET, id, TOKEN_TTL).unwrap();
        assert_eq!(verify_token(SECRET, &token).unwrap(), id);
    }

    #[test]
    fn rejects_wrong_secret() {
        let id = Uuid::new_v4();
        let token = create_token(SECRET, id, TOKEN_TTL).unwrap();
        assert!(verify_token("other-secret", &token).is_err());
    }

    #[test]
    fn rejects_expired_token() {
        let id = Uuid::new_v4();
        // Well beyond the default clock-skew leeway (60s).
        let token = create_token(SECRET, id, Duration::seconds(-120)).unwrap();
        assert!(verify_token(SECRET, &token).is_err());
    }

    #[test]
    fn rejects_garbage() {
        assert!(verify_token(SECRET, "not.a.jwt").is_err());
    }

    #[test]
    fn rejects_token_signed_with_a_different_algorithm() {
        // A token whose header claims HS384 must not validate under our
        // HS256-only policy (algorithm pinning / confusion defense).
        let now = Utc::now();
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            iat: now.timestamp(),
            exp: (now + TOKEN_TTL).timestamp(),
        };
        let token = encode(
            &Header::new(Algorithm::HS384),
            &claims,
            &EncodingKey::from_secret(SECRET.as_bytes()),
        )
        .unwrap();
        assert!(verify_token(SECRET, &token).is_err());
    }
}
