//! Typed error enum for jobsmith.
//!
//! Every public function that can fail returns a specific `JobsmithError` variant.
//! `anyhow` is banned from the public API per AGENTS.md.

use std::io;

/// The top-level error type for jobsmith.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum JobsmithError {
    /// HTTP request to HeadHunter API failed.
    #[error("hh api request failed: {0}")]
    HhApiRequest(#[from] reqwest::Error),

    /// HeadHunter API returned a non-success status code.
    #[error("hh api returned status {status}: {message}")]
    HhApiStatus { status: u16, message: String },

    /// Database operation failed.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// I/O operation failed.
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    /// Workflow encountered an invalid state transition.
    #[error("invalid workflow state: expected {expected}, got {actual}")]
    InvalidWorkflowState { expected: String, actual: String },

    /// Template compilation failed (e.g., Typst returned an error).
    #[error("template compilation failed: {0}")]
    TemplateCompilation(String),

    /// Template rendering failed (missing field, invalid syntax).
    #[error("template rendering failed: {0}")]
    TemplateRender(String),

    /// Configuration file is missing or invalid.
    #[error("configuration error: {0}")]
    Config(String),

    /// Salary lookup returned no results.
    #[error("salary lookup found no matches for '{query}'")]
    SalaryNotFound { query: String },

    /// JSON serialization or deserialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Profile validation failed (required field missing).
    #[error("profile validation failed: {0}")]
    ProfileValidation(String),

    /// External process execution failed (e.g., typst-cli, kimi).
    #[error("process execution failed: {0}")]
    Process(String),

    /// AI response could not be parsed into the expected structure.
    #[error("response parse failed: {0}")]
    ResponseParse(String),

    /// External process timed out.
    #[error("process timed out after {duration_secs}s")]
    ProcessTimeout { duration_secs: u64 },

    /// Kimi agent protocol (ACP) error.
    #[error("kimi protocol error: {0}")]
    KimiProtocol(String),

    /// Invalid vacancy ID provided.
    #[error("invalid vacancy id: {0}")]
    InvalidVacancyId(String),

    /// Invalid application status string.
    #[error("invalid application status: {0}")]
    InvalidApplicationStatus(String),

    /// Database migration failed.
    #[error("migration failed at version {version}: {detail}")]
    Migration { version: u32, detail: String },

    /// Fit score is below the minimum acceptable threshold.
    #[error("fit score {score} is below the minimum acceptable threshold of {min}")]
    FitScoreTooLow { score: i32, min: i32 },

    /// User cancelled the operation.
    #[error("operation cancelled: {0}")]
    Cancelled(String),
}

/// Convenience type alias for Results in jobsmith.
pub type Result<T> = std::result::Result<T, JobsmithError>;
