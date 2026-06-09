//! Candidate profile management.
//!
//! Profiles are stored in SQLite with migrations.

pub mod model;
pub mod store;

pub use model::*;
pub use store::{ApplicationRecord, ApplicationStatus, ProfileStore};
