//! Workflow orchestration for job applications.
//!
//! Manages the drafter-reviewer pipeline via Kimi Code.

pub mod client;
pub mod kimi;
pub mod parser;
pub mod prompts;
pub mod state;

pub use client::WorkflowClient;
pub use kimi::KimiClient;
pub use state::{Stage, WorkflowEngine};
