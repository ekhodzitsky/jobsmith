//! SQL schema migrations.

use rusqlite::Connection;

use crate::error::{JobsmithError, Result};

/// A single SQL schema migration.
pub struct SqlMigration {
    /// Monotonically increasing version number.
    pub version: u32,
    /// SQL script to execute.
    pub sql: &'static str,
}

/// Runs SQL migrations idempotently inside transactions.
pub struct MigrationRunner;

impl MigrationRunner {
    /// Apply all migrations with version greater than the current DB version.
    pub fn run(conn: &mut Connection, migrations: &[SqlMigration]) -> Result<()> {
        // Bootstrap the metadata table.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .map_err(JobsmithError::Database)?;

        let current: u32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM _schema_migrations",
                [],
                |row| row.get(0),
            )
            .map_err(JobsmithError::Database)?;

        for migration in migrations {
            if migration.version > current {
                let tx = conn.transaction().map_err(JobsmithError::Database)?;
                tx.execute_batch(migration.sql)
                    .map_err(|e| JobsmithError::Migration {
                        version: migration.version,
                        source: e.to_string(),
                    })?;
                tx.execute(
                    "INSERT INTO _schema_migrations(version) VALUES (?1)",
                    rusqlite::params![migration.version],
                )
                .map_err(JobsmithError::Database)?;
                tx.commit().map_err(JobsmithError::Database)?;
            }
        }

        Ok(())
    }
}

/// Registry of all SQL schema migrations.
/// Add new entries at the end. Versions must be strictly monotonic.
pub static ALL_SQL_MIGRATIONS: &[SqlMigration] = &[
    // v1: baseline — metadata table is created by the runner itself.
    SqlMigration {
        version: 1,
        sql: "",
    },
];
