//! HTTP route tree, mounted under `/api`.

use axum::Router;

use crate::AppState;

pub mod auth;

pub fn router() -> Router<AppState> {
    Router::new().merge(auth::router())
}
