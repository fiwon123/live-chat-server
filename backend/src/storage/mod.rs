//! Data-access layer: compile-time-checked sqlx queries against Postgres.

pub mod messages;
pub mod reactions;
pub mod room_memberships;
pub mod rooms;
pub mod users;

pub use messages::{MessageWithAuthor, Messages};
pub use reactions::{ReactionGroup, ReactionRow, Reactions, group_by_message};
pub use room_memberships::RoomMemberships;
pub use rooms::{RoomWithMembership, Rooms};
pub use users::Users;
