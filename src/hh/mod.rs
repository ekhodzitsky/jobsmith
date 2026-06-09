//! HeadHunter API client and models.
//!
//! Uses the public HH API (api.hh.ru) with forward-compatible serde models.

pub mod client;
pub mod models;

pub use client::{extract_vacancy_id, HhClient};
pub use models::*;
