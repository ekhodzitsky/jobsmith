//! Workflow orchestration for job applications.
//!
//! Manages the drafter-reviewer pipeline via Kimi Code.

pub mod acp;
pub mod client;
pub mod parser;
pub mod prompts;
pub mod state;

pub use acp::AcpClient;
pub use client::WorkflowClient;
pub use state::{Stage, WorkflowEngine};
