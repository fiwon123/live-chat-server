//! Registration and login logic.

use uuid::Uuid;

use crate::auth::jwt;
use crate::auth::password;
use crate::auth::validation::{self, MAX_USERNAME_LEN};
use crate::error::AppError;
use crate::models::User;
use crate::storage::Users;

/// A verified user plus a freshly minted access token.
pub struct Authenticated {
    pub user: User,
    pub token: String,
}

/// Auth use-cases over the user store.
#[derive(Clone)]
pub struct AuthService {
    users: Users,
    jwt_secret: String,
}

impl AuthService {
    pub fn new(users: Users, jwt_secret: String) -> Self {
        Self { users, jwt_secret }
    }

    /// Create a user and return it with a signed token.
    ///
    /// Returns [`AppError::Conflict`] if the username is taken, both from a
    /// pre-check (friendly path) and from the unique constraint (race-safe).
    pub async fn register(
        &self,
        username: &str,
        plaintext: &str,
    ) -> Result<Authenticated, AppError> {
        validation::validate_username(username)?;
        validation::validate_password(plaintext)?;

        if self.users.find_by_username(username).await?.is_some() {
            return Err(AppError::Conflict("username is already taken".to_string()));
        }

        let password_hash = password::hash_password(plaintext)?;
        let user = self
            .users
            .create(username, &password_hash)
            .await
            .map_err(|err| {
                if AppError::is_unique_violation(&err) {
                    AppError::Conflict("username is already taken".to_string())
                } else {
                    AppError::Internal(err.into())
                }
            })?;

        tracing::info!(user_id = %user.id, "user registered");
        let token = self.issue_token(user.id)?;
        Ok(Authenticated { user, token })
    }

    /// Verify credentials and return the user with a signed token.
    ///
    /// A missing user and a wrong password return the same error, and the
    /// missing-user path still performs a password verification against a
    /// dummy hash so response time does not reveal whether the user exists.
    pub async fn login(&self, username: &str, plaintext: &str) -> Result<Authenticated, AppError> {
        // Bound the work before touching the database or argon2: reject
        // over-long input with the same generic error as a wrong password.
        if username.chars().count() > MAX_USERNAME_LEN
            || plaintext.chars().count() > validation::MAX_PASSWORD_LEN
        {
            return Err(invalid_credentials());
        }

        match self.users.find_by_username(username).await? {
            Some(user) if password::verify_password(plaintext, &user.password_hash) => {
                let token = self.issue_token(user.id)?;
                Ok(Authenticated { user, token })
            }
            Some(_) => Err(invalid_credentials()),
            None => {
                // Equalize work with the verifying path; result is discarded.
                let _ = password::verify_password(plaintext, dummy_hash());
                Err(invalid_credentials())
            }
        }
    }

    fn issue_token(&self, user_id: Uuid) -> Result<String, AppError> {
        jwt::create_token(&self.jwt_secret, user_id, jwt::TOKEN_TTL)
    }
}

/// Same message for unknown user and wrong password — no enumeration oracle.
fn invalid_credentials() -> AppError {
    AppError::Unauthorized("invalid username or password".to_string())
}

/// A real argon2id hash used only to burn comparable CPU time when the
/// username is unknown. Computed once, from a throwaway password.
fn dummy_hash() -> &'static str {
    use std::sync::OnceLock;
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| password::hash_password("timing-equalizer").unwrap_or_default())
}
