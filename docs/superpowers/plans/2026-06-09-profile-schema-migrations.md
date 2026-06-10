# Profile Schema Versioning & Migrations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a built-in SQL and JSON migration framework to `ProfileStore` so that future schema changes are applied automatically, safely, and transactionally.

**Architecture:** A synchronous `MigrationRunner` for SQL schema changes (tracking versions in `_schema_migrations`) and a `JsonMigrationRunner` for profile JSON structure changes (tracking versions via a `schema_version` field inside the JSON blob). Both are invoked from `ProfileStore::init()` and `load_profile()` respectively.

**Tech Stack:** Rust 2021, `rusqlite`, `serde_json`, `tokio::sync::Mutex`, existing `JobsmithError` infrastructure.

---

## File Structure

| File | Responsibility |
|------|----------------|
| `src/error.rs` | Add `Migration` error variant. |
| `src/profile/model.rs` | Add `schema_version: u32` to `Profile` with serde default. |
| `src/profile/mod.rs` | Declare `pub mod migrations`. |
| `src/profile/migrations.rs` | Module root: re-export public types from `sql` and `json` submodules. |
| `src/profile/migrations/sql.rs` | `SqlMigration`, `MigrationRunner`, `ALL_SQL_MIGRATIONS`. |
| `src/profile/migrations/json.rs` | `JsonProfileMigration`, `JsonMigrationRunner`, `ALL_JSON_MIGRATIONS`, `CURRENT_PROFILE_SCHEMA`. |
| `src/profile/store.rs` | Wire migration runner into `init()`, `load_profile()`, and `save_profile()`. Add `#[cfg(test)] insert_raw_profile` helper. |
| `tests/profile_migrations_test.rs` | Integration tests for SQL and JSON migration runners and auto-migration on load. |

---

### Task 1: Add `Migration` Error Variant

**Files:**
- Modify: `src/error.rs`

- [ ] **Step 1: Add the variant**

Insert the new variant between `InvalidApplicationStatus` and `FitScoreTooLow`:

```rust
    /// Database migration failed.
    #[error("migration failed at version {version}: {source}")]
    Migration { version: u32, source: String },
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check --all-features`
Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add src/error.rs
git commit -m "feat: add Migration error variant"
```

---

### Task 2: Add `schema_version` to `Profile`

**Files:**
- Modify: `src/profile/model.rs`

- [ ] **Step 1: Add the field and default helper**

After the imports and before `pub struct Profile`, add:

```rust
pub const CURRENT_PROFILE_SCHEMA: u32 = 1;

fn default_schema_version() -> u32 {
    CURRENT_PROFILE_SCHEMA
}
```

Inside `Profile`, add as the **first** field:

```rust
    /// JSON schema version for automatic migrations.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
```

- [ ] **Step 2: Verify tests still compile**

Run: `cargo test --all-features --no-run`
Expected: Compilation succeeds (existing tests may need to update struct literals).

- [ ] **Step 3: Commit**

```bash
git add src/profile/model.rs
git commit -m "feat: add schema_version field to Profile"
```

---

### Task 3: Create Migration Module Files

**Files:**
- Create: `src/profile/migrations.rs`
- Create: `src/profile/migrations/sql.rs`
- Create: `src/profile/migrations/json.rs`
- Modify: `src/profile/mod.rs`

- [ ] **Step 1: Declare module in `src/profile/mod.rs`**

Change:
```rust
pub mod model;
pub mod store;
```
to:
```rust
pub mod migrations;
pub mod model;
pub mod store;
```

- [ ] **Step 2: Create `src/profile/migrations.rs`**

```rust
//! Built-in migration framework for SQLite schema and profile JSON.

pub mod json;
pub mod sql;

pub use json::{JsonMigrationRunner, JsonProfileMigration, ALL_JSON_MIGRATIONS, CURRENT_PROFILE_SCHEMA};
pub use sql::{MigrationRunner, SqlMigration, ALL_SQL_MIGRATIONS};
```

- [ ] **Step 3: Create `src/profile/migrations/sql.rs`**

```rust
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
    pub fn run(conn: &Connection, migrations: &[SqlMigration]) -> Result<()> {
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
                tx.execute_batch(migration.sql).map_err(|e| {
                    JobsmithError::Migration {
                        version: migration.version,
                        source: e.to_string(),
                    }
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
```

- [ ] **Step 4: Create `src/profile/migrations/json.rs`**

```rust
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
        let mut version = value
            .get("schema_version")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(0);

        let mut value = value;

        while version < CURRENT_PROFILE_SCHEMA {
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
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check --all-features`
Expected: No errors.

- [ ] **Step 6: Commit**

```bash
git add src/profile/mod.rs src/profile/migrations.rs src/profile/migrations/sql.rs src/profile/migrations/json.rs
git commit -m "feat: add built-in SQL and JSON migration framework"
```

---

### Task 4: Integrate SQL Migrations into `ProfileStore::init()`

**Files:**
- Modify: `src/profile/store.rs`

- [ ] **Step 1: Add import and call `MigrationRunner::run`**

Inside `ProfileStore::init()`, after the existing `conn.execute_batch(...)` and before `.map_err(JobsmithError::Database)?;`, insert:

```rust
        crate::profile::migrations::sql::MigrationRunner::run(
            &conn,
            crate::profile::migrations::sql::ALL_SQL_MIGRATIONS,
        )?;
```

The full `init` body should look like:

```rust
    async fn init(&self) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute_batch(
            r#"
                CREATE TABLE IF NOT EXISTS profiles (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    data TEXT NOT NULL,
                    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE TABLE IF NOT EXISTS applications (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    vacancy_id TEXT NOT NULL,
                    vacancy_name TEXT,
                    employer_name TEXT,
                    status TEXT NOT NULL DEFAULT 'draft',
                    fit_score INTEGER,
                    cv_path TEXT,
                    cover_path TEXT,
                    notes TEXT,
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
                );

                CREATE INDEX IF NOT EXISTS idx_applications_vacancy
                    ON applications(vacancy_id);
                "#,
        )
        .map_err(JobsmithError::Database)?;

        crate::profile::migrations::sql::MigrationRunner::run(
            &conn,
            crate::profile::migrations::sql::ALL_SQL_MIGRATIONS,
        )?;

        Ok(())
    }
```

- [ ] **Step 2: Verify tests pass**

Run: `cargo test --all-features`
Expected: All existing tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/profile/store.rs
git commit -m "feat: integrate SQL migrations into ProfileStore::init"
```

---

### Task 5: Integrate JSON Migrations into `ProfileStore::load_profile()` and `save_profile()`

**Files:**
- Modify: `src/profile/store.rs`

- [ ] **Step 1: Rewrite `load_profile` to migrate JSON inline**

Replace the existing `load_profile` method with:

```rust
    /// Load the candidate profile, applying JSON migrations automatically.
    #[instrument(skip(self))]
    pub async fn load_profile(&self) -> Result<Option<Profile>> {
        let conn = self.conn.lock().await;
        let data: Option<String> = conn
            .query_row("SELECT data FROM profiles WHERE id = 1", [], |row| row.get(0))
            .optional()
            .map_err(JobsmithError::Database)?;

        match data {
            Some(json) => {
                let value: serde_json::Value =
                    serde_json::from_str(&json).map_err(JobsmithError::Json)?;

                let original_version = value
                    .get("schema_version")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or(0);

                let migrated = crate::profile::migrations::json::JsonMigrationRunner::run(
                    value,
                    crate::profile::migrations::json::ALL_JSON_MIGRATIONS,
                )?;

                let new_version = migrated
                    .get("schema_version")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32)
                    .unwrap_or(0);

                // Persist migrated JSON immediately while still holding the lock.
                if new_version != original_version {
                    let updated_json =
                        serde_json::to_string(&migrated).map_err(JobsmithError::Json)?;
                    conn.execute(
                        "UPDATE profiles SET data = ?1, updated_at = datetime('now') WHERE id = 1",
                        rusqlite::params![updated_json],
                    )
                    .map_err(JobsmithError::Database)?;
                }

                let profile = serde_json::from_value(migrated).map_err(JobsmithError::Json)?;
                debug!("profile loaded");
                Ok(Some(profile))
            }
            None => Ok(None),
        }
    }
```

- [ ] **Step 2: Update `save_profile` to stamp `schema_version`**

Replace the existing `save_profile` method with:

```rust
    /// Save or update the candidate profile.
    #[instrument(skip(self, profile))]
    pub async fn save_profile(&self, profile: &Profile) -> Result<()> {
        let mut value = serde_json::to_value(profile).map_err(JobsmithError::Json)?;
        value["schema_version"] =
            serde_json::json!(crate::profile::migrations::json::CURRENT_PROFILE_SCHEMA);
        let data = serde_json::to_string(&value).map_err(JobsmithError::Json)?;

        let conn = self.conn.lock().await;
        conn.execute(
            r#"
                INSERT INTO profiles (id, data, updated_at)
                VALUES (1, ?1, datetime('now'))
                ON CONFLICT(id) DO UPDATE SET
                    data = excluded.data,
                    updated_at = excluded.updated_at
                "#,
            rusqlite::params![data],
        )
        .map_err(JobsmithError::Database)?;

        info!("profile saved");
        Ok(())
    }
```

- [ ] **Step 3: Add test helper `insert_raw_profile`**

Append at the end of `src/profile/store.rs`, before the `ApplicationRecord` struct:

```rust
#[cfg(test)]
impl ProfileStore {
    /// Insert a raw JSON profile bypassing serialization (for migration tests).
    pub async fn insert_raw_profile(&self, json: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO profiles (id, data, updated_at) VALUES (1, ?1, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET data = excluded.data, updated_at = excluded.updated_at",
            rusqlite::params![json],
        )
        .map_err(JobsmithError::Database)?;
        Ok(())
    }
}
```

- [ ] **Step 4: Verify tests pass**

Run: `cargo test --all-features`
Expected: All existing tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/profile/store.rs
git commit -m "feat: integrate JSON migrations into ProfileStore load/save"
```

---

### Task 6: Write Migration Tests

**Files:**
- Create: `tests/profile_migrations_test.rs`

- [ ] **Step 1: Create test file**

```rust
//! Integration tests for the profile migration framework.

use jobsmith::profile::migrations::json::{
    JsonMigrationRunner, JsonProfileMigration, ALL_JSON_MIGRATIONS, CURRENT_PROFILE_SCHEMA,
};
use jobsmith::profile::migrations::sql::{MigrationRunner, SqlMigration, ALL_SQL_MIGRATIONS};
use jobsmith::profile::model::Profile;
use jobsmith::profile::store::ProfileStore;

// ---------------------------------------------------------------------------
// SQL migrations
// ---------------------------------------------------------------------------

#[test]
fn test_sql_migration_applies_pending() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let migrations = &[
        SqlMigration {
            version: 1,
            sql: "CREATE TABLE test_table (id INTEGER PRIMARY KEY);",
        },
        SqlMigration {
            version: 2,
            sql: "ALTER TABLE test_table ADD COLUMN name TEXT;",
        },
    ];
    MigrationRunner::run(&conn, migrations).unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM _schema_migrations",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_sql_migration_idempotent() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let migrations = &[SqlMigration {
        version: 1,
        sql: "CREATE TABLE test_table (id INTEGER PRIMARY KEY);",
    }];
    MigrationRunner::run(&conn, migrations).unwrap();
    MigrationRunner::run(&conn, migrations).unwrap(); // second run must not fail

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM _schema_migrations WHERE version = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_sql_migration_fails_on_bad_sql() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let migrations = &[SqlMigration {
        version: 1,
        sql: "THIS IS NOT VALID SQL;",
    }];
    let result = MigrationRunner::run(&conn, migrations);
    assert!(result.is_err(), "bad SQL should fail");
}

#[test]
fn test_sql_baseline_registry_runs_cleanly() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    // The real registry should apply without error on a fresh DB.
    MigrationRunner::run(&conn, ALL_SQL_MIGRATIONS).unwrap();
}

// ---------------------------------------------------------------------------
// JSON migrations
// ---------------------------------------------------------------------------

#[test]
fn test_json_migration_v0_to_v1() {
    let value = serde_json::json!({
        "name": "Alice",
        "city": "Moscow",
        "phone": "+7...",
        "email": "alice@example.com"
    });
    let migrated = JsonMigrationRunner::run(value, ALL_JSON_MIGRATIONS).unwrap();
    assert_eq!(migrated["schema_version"], 1);
}

#[test]
fn test_json_migration_chain() {
    let migrations = &[
        JsonProfileMigration {
            from_version: 0,
            to_version: 1,
            migrate: |mut v| {
                v["schema_version"] = serde_json::json!(1);
                Ok(v)
            },
        },
        JsonProfileMigration {
            from_version: 1,
            to_version: 2,
            migrate: |mut v| {
                v["new_field"] = serde_json::json!("migrated");
                Ok(v)
            },
        },
    ];
    let value = serde_json::json!({ "name": "Bob" });
    let migrated = JsonMigrationRunner::run(value, migrations).unwrap();
    assert_eq!(migrated["schema_version"], 1);
    assert_eq!(migrated["new_field"], "migrated");
}

#[test]
fn test_json_migration_missing_step() {
    let migrations = &[JsonProfileMigration {
        from_version: 0,
        to_version: 2, // skips version 1
        migrate: |v| Ok(v),
    }];
    let value = serde_json::json!({ "schema_version": 0 });
    let result = JsonMigrationRunner::run(value, migrations);
    assert!(result.is_err(), "broken chain should fail");
}

#[test]
fn test_json_migration_already_current() {
    let value = serde_json::json!({
        "schema_version": CURRENT_PROFILE_SCHEMA,
        "name": "Charlie"
    });
    let migrated = JsonMigrationRunner::run(value, ALL_JSON_MIGRATIONS).unwrap();
    assert_eq!(migrated["schema_version"], CURRENT_PROFILE_SCHEMA);
}

// ---------------------------------------------------------------------------
// Integration: auto-migration on load
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_load_profile_auto_migrates_v0_to_v1() {
    let store = ProfileStore::open_in_memory().await.unwrap();

    // Insert legacy JSON without schema_version.
    let legacy_json = r#"{"name":"Dana","city":"SPb","phone":"+7...","email":"dana@example.com"}"#;
    store.insert_raw_profile(legacy_json).await.unwrap();

    // Load should auto-migrate and return a valid Profile.
    let profile = store.load_profile().await.unwrap().expect("profile should exist");
    assert_eq!(profile.schema_version, CURRENT_PROFILE_SCHEMA);
    assert_eq!(profile.name, "Dana");

    // Second load should be a no-op (already migrated).
    let profile2 = store.load_profile().await.unwrap().expect("profile should exist");
    assert_eq!(profile2.schema_version, CURRENT_PROFILE_SCHEMA);
}

#[tokio::test]
async fn test_save_profile_sets_schema_version() {
    let store = ProfileStore::open_in_memory().await.unwrap();

    let profile = Profile {
        name: "Eve".to_string(),
        city: "Kazan".to_string(),
        phone: "+7...".to_string(),
        email: "eve@example.com".to_string(),
        ..Default::default()
    };
    store.save_profile(&profile).await.unwrap();

    let loaded = store.load_profile().await.unwrap().expect("profile should exist");
    assert_eq!(loaded.schema_version, CURRENT_PROFILE_SCHEMA);
}
```

- [ ] **Step 2: Run new tests**

Run: `cargo test --all-features profile_migrations`
Expected: All 8 tests pass.

- [ ] **Step 3: Commit**

```bash
git add tests/profile_migrations_test.rs
git commit -m "test: add profile migration integration tests"
```

---

### Task 7: Final Verification

- [ ] **Step 1: Run full test suite**

Run: `cargo test --all-features`
Expected: All tests pass (existing + 8 new).

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features`
Expected: No errors, no warnings.

- [ ] **Step 3: Commit if clean**

```bash
git add -A
git diff --cached --quiet || git commit -m "chore: profile schema versioning complete"
```

---

## Plan Self-Review

### Spec Coverage

| Spec Requirement | Task |
|------------------|------|
| `Migration` error variant | Task 1 |
| `schema_version` in `Profile` | Task 2 |
| `_schema_migrations` table | Task 3 (sql.rs) |
| `MigrationRunner` for SQL | Task 3 |
| `JsonMigrationRunner` | Task 3 (json.rs) |
| Integration into `ProfileStore::init()` | Task 4 |
| Integration into `load_profile()` | Task 5 |
| Auto-save migrated JSON (same lock) | Task 5 |
| `save_profile` stamps version | Task 5 |
| Test: SQL applies pending | Task 6 |
| Test: SQL idempotent | Task 6 |
| Test: SQL fails on bad SQL | Task 6 |
| Test: JSON v0→v1 | Task 6 |
| Test: JSON chain | Task 6 |
| Test: JSON missing step | Task 6 |
| Test: load auto-migrates | Task 6 |

**Gaps:** None.

### Placeholder Scan

No `TBD`, `TODO`, "implement later", or vague instructions found.

### Type Consistency

- `schema_version: u32` used everywhere (model, JSON runner, error variant).
- `MigrationRunner::run` signature matches in definition (Task 3) and call sites (Task 4).
- `JsonMigrationRunner::run` signature matches in definition (Task 3) and call site (Task 5).
- `CURRENT_PROFILE_SCHEMA` exported from `json.rs` and used in `save_profile` (Task 5).
