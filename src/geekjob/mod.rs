//! GeekJob integration.
//!
//! Reads vacancies from GeekJob's public pages: the search listing for
//! discovery and the schema.org `JobPosting` JSON-LD block (shared
//! crate-private extractor) for details. `robots.txt` permits these
//! paths and nothing bypasses access control.

pub mod client;
pub mod detail;
pub mod listing;

pub use client::GeekjobClient;
