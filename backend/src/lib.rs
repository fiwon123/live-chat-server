//! live-chat-server library crate.
//!
//! The binary is a thin shell around the pieces assembled here so that
//! integration tests can build and serve the application without spawning a
//! subprocess.

pub mod auth;
pub mod config;
pub mod error;
pub mod models;
pub mod routes;
pub mod services;
pub mod storage;
pub mod ws;

use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::get};
use sqlx::PgPool;

use crate::config::Config;
use crate::ws::RoomRegistry;

/// Application-wide shared state, cloned into every handler.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    /// Per-room WebSocket registry; cheap to clone (its internals are already
    /// `Arc`-shared). In-memory by default, Redis Pub/Sub behind `with_redis`.
    pub registry: RoomRegistry,
}

impl AppState {
    /// In-memory broadcast registry (single server instance).
    pub fn new(pool: PgPool, config: Config) -> Self {
        Self {
            pool,
            config: Arc::new(config),
            registry: RoomRegistry::new(),
        }
    }

    /// Broadcast room events through Redis Pub/Sub (multi-instance). The
    /// caller is expected to have already verified the connection; `connect`
    /// failures fall back to [`AppState::new`] with a logged warning.
    pub fn with_redis(pool: PgPool, config: Config, redis: crate::ws::RedisPubSub) -> Self {
        Self {
            pool,
            config: Arc::new(config),
            registry: RoomRegistry::with_redis(redis),
        }
    }
}

/// Build the application router with the given shared state.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .nest("/api", routes::router())
        .with_state(state)
}

/// Liveness probe: returns `200 OK` with a JSON body.
async fn health(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "live-chat-server",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
