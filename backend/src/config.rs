//! Application configuration loaded from the environment.
//!
//! Defaults mirror `../.env.example` so the server runs out of the box in the
//! dev sandbox, where the compose stack injects real values.

use std::env;

/// Runtime configuration for the API server.
#[derive(Debug, Clone)]
pub struct Config {
    /// TCP port the HTTP/WebSocket server binds to.
    pub port: u16,
    /// PostgreSQL connection string (unused until the storage layer lands).
    pub database_url: String,
    /// Secret used to sign JWTs.
    pub jwt_secret: String,
    /// Redis URL for cross-instance pub/sub broadcasting. `None` keeps room
    /// fan-out fully in-memory (single server instance).
    pub redis_url: Option<String>,
}

/// Placeholder secret used when `JWT_SECRET` is not set.
///
/// Clearly insecure and committed in the repo, so tokens signed with it can be
/// forged by anyone. Fine for local development; the server logs a warning
/// when it is in use, and it must be overridden anywhere else.
pub const DEFAULT_JWT_SECRET: &str = "dev-only-insecure-secret-change-me";

impl Config {
    /// Load configuration from environment variables, falling back to the
    /// defaults in `.env.example`.
    pub fn from_env() -> Self {
        Self::from_values(
            env::var("PORT").ok(),
            env::var("DATABASE_URL").ok(),
            env::var("JWT_SECRET").ok(),
            env::var("REDIS_URL").ok(),
        )
    }

    /// True when the signing secret is still the committed placeholder.
    pub fn uses_default_jwt_secret(&self) -> bool {
        self.jwt_secret == DEFAULT_JWT_SECRET
    }

    /// Build config from optional overrides; `None` selects the default.
    ///
    /// Kept separate from [`Config::from_env`] so the fallback logic is
    /// testable without touching the process environment.
    fn from_values(
        port: Option<String>,
        database_url: Option<String>,
        jwt_secret: Option<String>,
        redis_url: Option<String>,
    ) -> Self {
        let port = port.and_then(|v| v.parse().ok()).unwrap_or(8080);
        let database_url = database_url
            .unwrap_or_else(|| "postgres://postgres:postgres@localhost:5434/live_chat".to_string());
        let jwt_secret = jwt_secret.unwrap_or_else(|| DEFAULT_JWT_SECRET.to_string());
        // An empty REDIS_URL counts as unset: users can disable pub/sub with
        // `REDIS_URL=`.
        let redis_url = redis_url.filter(|url| !url.is_empty());

        Self {
            port,
            database_url,
            jwt_secret,
            redis_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_fall_back_to_environment_template_values() {
        let cfg = Config::from_values(None, None, None, None);
        assert_eq!(cfg.port, 8080);
        assert!(cfg.database_url.contains("localhost:5434/live_chat"));
        assert_eq!(cfg.jwt_secret, DEFAULT_JWT_SECRET);
        assert!(cfg.uses_default_jwt_secret());
        assert_eq!(
            cfg.redis_url, None,
            "no REDIS_URL means in-memory broadcast"
        );
    }

    #[test]
    fn environment_values_take_precedence() {
        let cfg = Config::from_values(
            Some("9090".to_string()),
            Some("postgres://user:pass@db.example.com/chat".to_string()),
            Some("super-secret".to_string()),
            Some("redis://localhost:6381".to_string()),
        );
        assert_eq!(cfg.port, 9090);
        assert_eq!(cfg.database_url, "postgres://user:pass@db.example.com/chat");
        assert_eq!(cfg.jwt_secret, "super-secret");
        assert!(!cfg.uses_default_jwt_secret());
        assert_eq!(cfg.redis_url.as_deref(), Some("redis://localhost:6381"));
    }

    #[test]
    fn invalid_port_falls_back_to_default() {
        let cfg = Config::from_values(Some("not-a-port".to_string()), None, None, None);
        assert_eq!(cfg.port, 8080);
    }

    #[test]
    fn empty_redis_url_counts_as_unset() {
        let cfg = Config::from_values(None, None, None, Some(String::new()));
        assert_eq!(cfg.redis_url, None);
    }
}
