//! Input validation for auth requests.

use crate::error::AppError;

/// Maximum accepted password length.
///
/// argon2 hashing cost grows with input; capping it bounds work per request.
pub const MAX_PASSWORD_LEN: usize = 128;

/// Minimum accepted password length.
pub const MIN_PASSWORD_LEN: usize = 8;

/// Validate a username: 3-32 chars of `[a-z0-9_]`.
pub fn validate_username(username: &str) -> Result<(), AppError> {
    let len = username.chars().count();
    if !(3..=32).contains(&len) {
        return Err(AppError::BadRequest(
            "username must be between 3 and 32 characters".to_string(),
        ));
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(AppError::BadRequest(
            "username may only contain lowercase letters, digits, and underscores".to_string(),
        ));
    }
    Ok(())
}

/// Validate a password length.
pub fn validate_password(password: &str) -> Result<(), AppError> {
    let len = password.chars().count();
    if len < MIN_PASSWORD_LEN {
        return Err(AppError::BadRequest(format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        )));
    }
    if len > MAX_PASSWORD_LEN {
        return Err(AppError::BadRequest(format!(
            "password must be at most {MAX_PASSWORD_LEN} characters"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_reasonable_username() {
        assert!(validate_username("alice_99").is_ok());
        assert!(validate_username("abc").is_ok());
    }

    #[test]
    fn rejects_bad_usernames() {
        assert!(validate_username("ab").is_err(), "too short");
        assert!(validate_username(&"a".repeat(33)).is_err(), "too long");
        assert!(validate_username("Alice").is_err(), "uppercase");
        assert!(validate_username("has space").is_err(), "space");
        assert!(validate_username("dash-name").is_err(), "dash");
        assert!(validate_username("").is_err(), "empty");
    }

    #[test]
    fn enforces_password_length() {
        assert!(validate_password("short").is_err());
        assert!(validate_password("longenough").is_ok());
        assert!(validate_password(&"x".repeat(MIN_PASSWORD_LEN)).is_ok());
        assert!(validate_password(&"x".repeat(MAX_PASSWORD_LEN)).is_ok());
        assert!(validate_password(&"x".repeat(MAX_PASSWORD_LEN + 1)).is_err());
    }
}
