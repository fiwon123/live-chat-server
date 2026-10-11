use sqlx::PgPool;
use uuid::Uuid;

use crate::models::User;

/// User persistence operations.
#[derive(Debug, Clone)]
pub struct Users {
    pool: PgPool,
}

impl Users {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new user. The caller is responsible for hashing the password.
    pub async fn create(&self, username: &str, password_hash: &str) -> Result<User, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"INSERT INTO users (username, password_hash)
               VALUES ($1, $2)
               RETURNING id, username, password_hash, created_at"#,
            username,
            password_hash
        )
        .fetch_one(&self.pool)
        .await
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"SELECT id, username, password_hash, created_at
               FROM users
               WHERE username = $1"#,
            username
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"SELECT id, username, password_hash, created_at
               FROM users
               WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
    }
}
