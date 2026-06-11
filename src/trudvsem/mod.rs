//! Trudvsem («Работа России») integration.
//!
//! Reads vacancies from the official open-data JSON API at
//! `opendata.trudvsem.ru` — a government service published explicitly
//! for machine consumption, no key required.
//!
//! Trudvsem data is mapped into the shared
//! [`crate::hh::models::VacancyDetail`] so the rest of the pipeline is
//! source-agnostic.

pub mod api;
pub mod client;

pub use client::TrudvsemClient;
