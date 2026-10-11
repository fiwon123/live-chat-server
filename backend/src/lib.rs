//! live-chat-server library crate.
//!
//! The binary is a thin shell around the pieces assembled here so that
//! integration tests can build and serve the application without spawning a
//! subprocess.

pub mod config;

use axum::{routing::get, Router};

/// Build the application router.
///
/// State is intentionally absent for now; the health check is stateless.
/// Later issues attach `AppState` (DB pool, room registry, auth config) here.
pub fn build_router() -> Router {
    Router::new().route("/healthz", get(healthz))
}

/// Liveness probe: returns `200 OK` with a JSON body.
async fn healthz() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "service": "live-chat-server",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
