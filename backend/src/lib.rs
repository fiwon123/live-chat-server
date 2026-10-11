//! live-chat-server library crate.
//!
//! The binary is a thin shell around the pieces assembled here so that
//! integration tests can build and serve the application without spawning a
//! subprocess.

pub mod config;
pub mod models;
pub mod storage;

use axum::{Router, extract::State, routing::get};
use sqlx::PgPool;

/// Application-wide shared state, cloned into every handler.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

/// Build the application router with the given shared state.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .with_state(state)
}

/// Liveness probe: returns `200 OK` with a JSON body.
async fn healthz(State(_state): State<AppState>) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "service": "live-chat-server",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
