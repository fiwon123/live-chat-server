//! Password hashing and verification (argon2id).

use anyhow::anyhow;
use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};

use crate::error::AppError;

/// Hash a plaintext password with argon2id and a random salt.
pub fn hash_password(password: &str) -> Result<String, AppError> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|err| AppError::Internal(anyhow!("failed to hash password: {err}")))
}

/// Verify a plaintext password against a stored PHC hash string.
///
/// Returns `false` for a malformed hash rather than erroring, so a corrupt
/// row cannot turn into an authentication bypass.
pub fn verify_password(password: &str, password_hash: &str) -> bool {
    match PasswordHash::new(password_hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_are_salted_and_verify() {
        let hash = hash_password("correct horse battery staple").unwrap();
        assert!(hash.starts_with("$argon2"));
        assert!(verify_password("correct horse battery staple", &hash));
        assert!(!verify_password("wrong password", &hash));
    }

    #[test]
    fn same_password_hashes_differently() {
        let a = hash_password("hunter2hunter2").unwrap();
        let b = hash_password("hunter2hunter2").unwrap();
        assert_ne!(a, b, "random salt should produce distinct hashes");
    }

    #[test]
    fn malformed_hash_does_not_verify() {
        assert!(!verify_password("anything", "not-a-phc-hash"));
    }
}
