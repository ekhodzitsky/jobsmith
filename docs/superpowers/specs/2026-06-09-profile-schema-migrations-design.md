# Profile Schema Versioning & Migrations Design

## Context

The `ProfileStore` persists candidate profiles as a single JSON blob in SQLite. Currently there is no versioning: if the `Profile` struct changes, existing database rows may fail to deserialize. We need a migration system that handles both SQL schema evolution and JSON structure evolution.

## Goals

1. Track database schema version and apply pending SQL migrations automatically on startup.
2. Track profile JSON schema version and apply pending JSON migrations automatically on load.
3. Keep migrations idempotent and transactional.
4. Zero new external dependencies.
5. Full test coverage for migration paths.

## Non-Goals

- Down-migrations (rollbacks). The application is single-user desktop software; downgrades are handled by restoring from backups.
- General-purpose migration framework for multiple databases. Scope is `ProfileStore` only.
- Async migrations. All migration code runs synchronously inside `rusqlite` transactions.

## Architecture

### New Files

- `src/profile/migrations.rs` — migration framework, registry, and runner.
- `src/profile/migrations/sql.rs` — SQL migration definitions.
- `src/profile/migrations/json.rs` — JSON profile migration definitions.

### Modified Files

- `src/profile/model.rs` — add `schema_version: u32` to `Profile`.
- `src/profile/store.rs` — integrate migration runner into `init()` and `load_profile()`.
- `src/error.rs` — add `Migration` error variant.

## SQL Schema Migrations

### Metadata Table

```sql
CREATE TABLE IF NOT EXISTS _schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### Migration Definition

```rust
pub struct SqlMigration {
    pub version: u32,
    pub sql: &'static str,
}
```

### Runner

`MigrationRunner::run(conn, migrations)` executes in this order:

1. Create `_schema_migrations` if not exists.
2. Query `SELECT MAX(version) FROM _schema_migrations`.
3. For each migration with `version > current`:
   a. Begin transaction.
   b. Execute `migration.sql`.
   c. `INSERT INTO _schema_migrations(version) VALUES (?1)`.
   d. Commit transaction.
4. If any step fails, rollback and return `JobsmithError::Migration`.

Idempotency guarantee: running `MigrationRunner` twice with the same registry is a no-op.

## Profile JSON Migrations

### Version Field

`Profile` gains a `schema_version: u32` field with a serde default so existing JSON without the field deserializes correctly:

```rust
pub const CURRENT_PROFILE_SCHEMA: u32 = 1;

fn default_schema_version() -> u32 {
    CURRENT_PROFILE_SCHEMA
}
```

In `Profile`:
```rust
#[serde(default = "default_schema_version")]
pub schema_version: u32,
```

### Migration Definition

```rust
pub struct JsonProfileMigration {
    pub from_version: u32,
    pub to_version: u32,
    pub migrate: fn(serde_json::Value) -> crate::Result<serde_json::Value>,
}
```

### Runner

`JsonMigrationRunner::run(value, migrations)`:

1. Extract `schema_version` from JSON. If missing, assume `0`.
2. While `version < CURRENT_PROFILE_SCHEMA`:
   a. Find migration where `from_version == version`.
   b. If none found, return error (broken migration chain).
   c. Apply `migrate` function.
   d. Set `"schema_version"` to `to_version`.
3. Return migrated `Value`.

### Initial Migration (v0 → v1)

For existing profiles without `schema_version`, the v0→v1 migration simply injects `"schema_version": 1` and returns. Future migrations will transform structure as needed.

## Integration Points

### `ProfileStore::init()`

After creating base tables (`profiles`, `applications`), call:

```rust
MigrationRunner::run(&conn, &ALL_SQL_MIGRATIONS)?;
```

### `ProfileStore::load_profile()`

After deserializing JSON string to `serde_json::Value`:

```rust
let migrated = JsonMigrationRunner::run(value, &ALL_JSON_MIGRATIONS)?;
let profile: Profile = serde_json::from_value(migrated)?;
```

If migrations were applied, the updated JSON is written back to the database using the **same locked connection** before releasing the mutex. This avoids a deadlock that would occur if `save_profile()` were called (it also acquires the same mutex).

### `ProfileStore::save_profile()`

Before serializing, ensure `schema_version` is set to `CURRENT_PROFILE_SCHEMA`.

## Error Handling

New error variant:

```rust
#[error("migration failed at version {version}: {source}")]
Migration { version: u32, source: String },
```

All migration failures are hard errors: the application refuses to start (SQL) or load profile (JSON) rather than risk data corruption.

## Testing Strategy

| Test | Description |
|------|-------------|
| `test_sql_migration_creates_metadata_table` | Fresh DB gets `_schema_migrations` table. |
| `test_sql_migration_applies_pending` | Registry with v1, v2 applies both. |
| `test_sql_migration_idempotent` | Second run does nothing. |
| `test_sql_migration_fails_on_bad_sql` | Invalid SQL returns `Migration` error. |
| `test_json_migration_v0_to_v1` | JSON without `schema_version` gets injected with `1`. |
| `test_json_migration_chain` | v0 → v1 → v2 applies sequentially. |
| `test_json_migration_missing_step` | Broken chain returns error. |
| `test_load_profile_auto_migrates` | `load_profile` on old JSON returns valid `Profile` and persists update. |

## Migration Registry (Initial)

```rust
pub static ALL_SQL_MIGRATIONS: &[SqlMigration] = &[
    SqlMigration {
        version: 1,
        sql: r#"
            CREATE TABLE IF NOT EXISTS _schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
        "#,
    },
];

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
```

Future schema changes add new entries to these static arrays. Versions are strictly monotonic.
