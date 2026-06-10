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
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
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
    MigrationRunner::run(&mut conn, migrations).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM _schema_migrations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn test_sql_migration_idempotent() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    let migrations = &[SqlMigration {
        version: 1,
        sql: "CREATE TABLE test_table (id INTEGER PRIMARY KEY);",
    }];
    MigrationRunner::run(&mut conn, migrations).unwrap();
    MigrationRunner::run(&mut conn, migrations).unwrap(); // second run must not fail

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
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    let migrations = &[SqlMigration {
        version: 1,
        sql: "THIS IS NOT VALID SQL;",
    }];
    let result = MigrationRunner::run(&mut conn, migrations);
    assert!(result.is_err(), "bad SQL should fail");
}

#[test]
fn test_sql_baseline_registry_runs_cleanly() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    // The real registry should apply without error on a fresh DB.
    MigrationRunner::run(&mut conn, ALL_SQL_MIGRATIONS).unwrap();
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
    let migrated = JsonMigrationRunner::run_to(value, migrations, 2).unwrap();
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
    // Current version is 1, but migrations only cover from 0.
    let value = serde_json::json!({ "schema_version": 1 });
    let result = JsonMigrationRunner::run_to(value, migrations, 2);
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

    // Build a full legacy JSON by serializing a default profile and removing schema_version.
    let mut legacy_value = serde_json::to_value(Profile::default()).unwrap();
    legacy_value
        .as_object_mut()
        .unwrap()
        .remove("schema_version");
    let legacy_json = serde_json::to_string(&legacy_value).unwrap();
    store.insert_raw_profile(&legacy_json).await.unwrap();

    // Load should auto-migrate and return a valid Profile.
    let profile = store
        .load_profile()
        .await
        .unwrap()
        .expect("profile should exist");
    assert_eq!(profile.schema_version, CURRENT_PROFILE_SCHEMA);
    assert_eq!(profile.name, Profile::default().name);

    // Second load should be a no-op (already migrated).
    let profile2 = store
        .load_profile()
        .await
        .unwrap()
        .expect("profile should exist");
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

    let loaded = store
        .load_profile()
        .await
        .unwrap()
        .expect("profile should exist");
    assert_eq!(loaded.schema_version, CURRENT_PROFILE_SCHEMA);
}
