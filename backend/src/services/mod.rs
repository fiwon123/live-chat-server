//! Business logic that sits between HTTP handlers and the storage layer.

pub mod auth;
pub mod rooms;

pub use auth::AuthService;
pub use rooms::{MarkReadResult, RoomService, RoomSummary};
