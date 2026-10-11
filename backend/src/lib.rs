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

use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::get};
use sqlx::PgPool;

use crate::config::Config;

/// Application-wide shared state, cloned into every handler.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
}

impl AppState {
    pub fn new(pool: PgPool, config: Config) -> Self {
        Self {
            pool,
            config: Arc::new(config),
        }
    }
}

/// Build the application router with the given shared state.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .nest("/api", routes::router())
        .with_state(state)
}

/// Liveness probe: returns `200 OK` with a JSON body.
async fn healthz(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "live-chat-server",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
