//! HTTP route tree, mounted under `/api`.

use axum::Router;

use crate::AppState;

pub mod auth;
pub mod rooms;

pub fn router() -> Router<AppState> {
    Router::new().merge(auth::router()).merge(rooms::router())
}
