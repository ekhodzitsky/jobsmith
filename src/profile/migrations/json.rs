//! JSON profile structure migrations.

use serde_json::Value;

use crate::error::{JobsmithError, Result};

/// Current profile JSON schema version.
pub const CURRENT_PROFILE_SCHEMA: u32 = 1;

/// A single JSON profile migration.
pub struct JsonProfileMigration {
    /// Version this migration starts from.
    pub from_version: u32,
    /// Version this migration produces.
    pub to_version: u32,
    /// Transform function.
    pub migrate: fn(Value) -> Result<Value>,
}

/// Runs JSON profile migrations sequentially.
pub struct JsonMigrationRunner;

impl JsonMigrationRunner {
    /// Migrate a JSON value up to `CURRENT_PROFILE_SCHEMA`.
    pub fn run(value: Value, migrations: &[JsonProfileMigration]) -> Result<Value> {
        Self::run_to(value, migrations, CURRENT_PROFILE_SCHEMA)
    }

    /// Migrate a JSON value up to an explicit target version.
    pub fn run_to(
        value: Value,
        migrations: &[JsonProfileMigration],
        target: u32,
    ) -> Result<Value> {
        let mut version = value
            .get("schema_version")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(0);

        let mut value = value;

        while version < target {
            let migration = migrations
                .iter()
                .find(|m| m.from_version == version)
                .ok_or_else(|| JobsmithError::Migration {
                    version,
                    source: format!("no migration found from version {version}"),
                })?;

            value = (migration.migrate)(value).map_err(|e| JobsmithError::Migration {
                version: migration.to_version,
                source: e.to_string(),
            })?;

            version = migration.to_version;
        }

        Ok(value)
    }
}

/// Registry of all JSON profile migrations.
pub static ALL_JSON_MIGRATIONS: &[JsonProfileMigration] = &[
    JsonProfileMigration {
        from_version: 0,
        to_version: 1,
        migrate: |mut v| {
            v["schema_version"] = serde_json::json!(1);
            Ok(v)
        },
    },
];
