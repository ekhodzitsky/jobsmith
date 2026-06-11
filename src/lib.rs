//! Jobsmith — AI-powered job application assistant for HeadHunter (Russia).
//!
//! Provides HeadHunter API client, profile management, workflow orchestration
//! via Kimi Code, and PDF document generation via Typst.

#![warn(clippy::await_holding_lock)]
#![warn(clippy::dbg_macro)]
#![warn(clippy::wildcard_imports)]
#![warn(clippy::unused_async)]
#![warn(clippy::missing_panics_doc)]
#![warn(clippy::cast_sign_loss)]

pub mod cli;
pub mod commands;
pub mod error;
pub mod hh;
pub(crate) mod http;
pub mod profile;
pub mod salary;
pub mod templates;
pub mod tui;
pub mod workflow;

pub use error::{JobsmithError, Result};
