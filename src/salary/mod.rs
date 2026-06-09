//! Salary benchmark lookup tool.
//!
//! Ported from the original Python `salary_lookup.py` to Rust.
//! Supports fuzzy matching with Russian-specific normalization.

pub mod lookup;

pub use lookup::SalaryLookup;
