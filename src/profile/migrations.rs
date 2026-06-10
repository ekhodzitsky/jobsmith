//! Built-in migration framework for SQLite schema and profile JSON.

pub mod json;
pub mod sql;

pub use json::{
    JsonMigrationRunner, JsonProfileMigration, ALL_JSON_MIGRATIONS, CURRENT_PROFILE_SCHEMA,
};
pub use sql::{MigrationRunner, SqlMigration, ALL_SQL_MIGRATIONS};
