//! SQLite-backed profile storage.

use std::fmt;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde_json;
use tokio::sync::Mutex;
use tracing::{debug, info, instrument};

use crate::error::{JobsmithError, Result};
use crate::profile::model::Profile;

/// The status of a job application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationStatus {
    /// Application is being drafted.
    Draft,
    /// Application has been submitted.
    Applied,
}

impl fmt::Display for ApplicationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplicationStatus::Draft => write!(f, "draft"),
            ApplicationStatus::Applied => write!(f, "applied"),
        }
    }
}

impl std::str::FromStr for ApplicationStatus {
    type Err = JobsmithError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "applied" => Ok(Self::Applied),
            _ => Err(JobsmithError::InvalidApplicationStatus(s.to_string())),
        }
    }
}

impl rusqlite::types::ToSql for ApplicationStatus {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::from(self.to_string()))
    }
}

impl rusqlite::types::FromSql for ApplicationStatus {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        value.as_str().and_then(|s| {
            s.parse::<Self>()
                .map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(e)))
        })
    }
}

/// Manages the SQLite database for profiles and application history.
#[derive(Debug)]
pub struct ProfileStore {
    conn: Mutex<Connection>,
}

impl ProfileStore {
    /// Open or create the profile database at the given path.
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path).map_err(JobsmithError::Database)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init().await?;
        Ok(store)
    }

    /// Open an in-memory database (useful for testing and quick prototyping).
    pub async fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(JobsmithError::Database)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init().await?;
        Ok(store)
    }

    /// Initialize the database schema.
    async fn init(&self) -> Result<()> {
        let mut conn = self.conn.lock().await;
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
            &mut conn,
            crate::profile::migrations::sql::ALL_SQL_MIGRATIONS,
        )?;

        Ok(())
    }

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
            params![data],
        )
        .map_err(JobsmithError::Database)?;

        info!("profile saved");
        Ok(())
    }

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

    /// Check if a profile exists.
    pub async fn has_profile(&self) -> Result<bool> {
        let conn = self.conn.lock().await;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM profiles WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(JobsmithError::Database)?;
        Ok(count > 0)
    }

    /// Record a new job application.
    #[instrument(skip(self), fields(vacancy_id = %vacancy_id))]
    pub async fn record_application(
        &self,
        vacancy_id: &str,
        vacancy_name: Option<&str>,
        employer_name: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn.lock().await;
        conn.execute(
            r#"
                INSERT INTO applications (vacancy_id, vacancy_name, employer_name)
                VALUES (?1, ?2, ?3)
                "#,
            params![vacancy_id, vacancy_name, employer_name],
        )
        .map_err(JobsmithError::Database)?;

        let id = conn.last_insert_rowid();
        info!(application_id = id, "application recorded");
        Ok(id)
    }

    /// Update application status.
    pub async fn update_application_status(
        &self,
        application_id: i64,
        status: ApplicationStatus,
        fit_score: Option<i32>,
        cv_path: Option<&str>,
        cover_path: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            r#"
                UPDATE applications
                SET status = ?1,
                    fit_score = ?2,
                    cv_path = ?3,
                    cover_path = ?4,
                    updated_at = datetime('now')
                WHERE id = ?5
                "#,
            params![status, fit_score, cv_path, cover_path, application_id],
        )
        .map_err(JobsmithError::Database)?;
        Ok(())
    }

    /// List all applications.
    pub async fn list_applications(&self) -> Result<Vec<ApplicationRecord>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, vacancy_id, vacancy_name, employer_name,
                       status, fit_score, cv_path, cover_path, notes,
                       created_at, updated_at
                FROM applications
                ORDER BY created_at DESC
                "#,
            )
            .map_err(JobsmithError::Database)?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ApplicationRecord {
                    id: row.get(0)?,
                    vacancy_id: row.get(1)?,
                    vacancy_name: row.get(2)?,
                    employer_name: row.get(3)?,
                    status: row.get(4)?,
                    fit_score: row.get(5)?,
                    cv_path: row.get(6)?,
                    cover_path: row.get(7)?,
                    notes: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(JobsmithError::Database)?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(JobsmithError::Database)
    }
}

impl ProfileStore {
    /// Insert a raw JSON profile bypassing serialization (for migration tests).
    #[doc(hidden)]
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

/// A recorded job application.
#[derive(Debug, Clone)]
pub struct ApplicationRecord {
    pub id: i64,
    pub vacancy_id: String,
    pub vacancy_name: Option<String>,
    pub employer_name: Option<String>,
    pub status: ApplicationStatus,
    pub fit_score: Option<i32>,
    pub cv_path: Option<String>,
    pub cover_path: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
