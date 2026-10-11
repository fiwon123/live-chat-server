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
    /// Secret used to sign JWTs (unused until auth lands).
    pub jwt_secret: String,
}

impl Config {
    /// Load configuration from environment variables, falling back to the
    /// defaults in `.env.example`.
    pub fn from_env() -> Self {
        Self::from_values(
            env::var("PORT").ok(),
            env::var("DATABASE_URL").ok(),
            env::var("JWT_SECRET").ok(),
        )
    }

    /// Build config from optional overrides; `None` selects the default.
    ///
    /// Kept separate from [`Config::from_env`] so the fallback logic is
    /// testable without touching the process environment.
    fn from_values(
        port: Option<String>,
        database_url: Option<String>,
        jwt_secret: Option<String>,
    ) -> Self {
        let port = port.and_then(|v| v.parse().ok()).unwrap_or(8080);
        let database_url = database_url
            .unwrap_or_else(|| "postgres://postgres:postgres@localhost:5434/live_chat".to_string());
        let jwt_secret =
            jwt_secret.unwrap_or_else(|| "dev-only-insecure-secret-change-me".to_string());

        Self {
            port,
            database_url,
            jwt_secret,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_fall_back_to_environment_template_values() {
        let cfg = Config::from_values(None, None, None);
        assert_eq!(cfg.port, 8080);
        assert!(cfg.database_url.contains("localhost:5434/live_chat"));
        assert_eq!(cfg.jwt_secret, "dev-only-insecure-secret-change-me");
    }

    #[test]
    fn environment_values_take_precedence() {
        let cfg = Config::from_values(
            Some("9090".to_string()),
            Some("postgres://user:pass@db.example.com/chat".to_string()),
            Some("super-secret".to_string()),
        );
        assert_eq!(cfg.port, 9090);
        assert_eq!(cfg.database_url, "postgres://user:pass@db.example.com/chat");
        assert_eq!(cfg.jwt_secret, "super-secret");
    }

    #[test]
    fn invalid_port_falls_back_to_default() {
        let cfg = Config::from_values(Some("not-a-port".to_string()), None, None);
        assert_eq!(cfg.port, 8080);
    }
}
