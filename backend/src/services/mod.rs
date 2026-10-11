//! Business logic that sits between HTTP handlers and the storage layer.

pub mod auth;

pub use auth::AuthService;
