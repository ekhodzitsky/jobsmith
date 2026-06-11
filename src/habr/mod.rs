//! Habr Career integration.
//!
//! Reads vacancies from Habr Career's public machine-readable channels:
//! the RSS search feed for listings and the schema.org `JobPosting`
//! JSON-LD block for full details. No protected endpoints are touched
//! and no anti-bot measures are bypassed (see `robots.txt`).
//!
//! Habr data is mapped into the shared [`crate::hh::models::VacancyDetail`]
//! so the rest of the pipeline (prompts, templates, workflow) is
//! source-agnostic.

pub mod client;
pub mod jsonld;
pub mod rss;

pub use client::HabrClient;
