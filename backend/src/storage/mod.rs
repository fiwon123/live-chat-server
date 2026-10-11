//! Data-access layer: compile-time-checked sqlx queries against Postgres.

pub mod messages;
pub mod room_memberships;
pub mod rooms;
pub mod users;

pub use messages::{MessageWithAuthor, Messages};
pub use room_memberships::RoomMemberships;
pub use rooms::{RoomWithMembership, Rooms};
pub use users::Users;
