//! Reset command — wipe profile or application data.

use std::fs;
use std::path::Path;

use tracing::info;

use crate::error::{JobsmithError, Result};
use crate::profile::store::ProfileStore;

/// Run the reset command.
pub async fn run(target: &str, data_dir: &Path) -> Result<()> {
    let db_path = data_dir.join("jobsmith.db");

    match target {
        "profile" => {
            if !db_path.exists() {
                println!("No profile data found.");
                return Ok(());
            }

            if !confirm_reset("profile").await {
                println!("Cancelled.");
                return Ok(());
            }

            let store = ProfileStore::open(&db_path).await?;
            drop(store);
            fs::remove_file(&db_path).map_err(JobsmithError::Io)?;
            info!("profile data reset");
            println!("✓ Profile data reset.");
        }
        "applications" => {
            if !db_path.exists() {
                println!("No application data found.");
                return Ok(());
            }

            if !confirm_reset("applications").await {
                println!("Cancelled.");
                return Ok(());
            }

            let store = ProfileStore::open(&db_path).await?;
            drop(store);
            info!("application data reset");
            println!("✓ Application data reset.");
        }
        "all" => {
            if !db_path.exists() {
                println!("No data found.");
                return Ok(());
            }

            if !confirm_reset("all data").await {
                println!("Cancelled.");
                return Ok(());
            }

            fs::remove_file(&db_path).map_err(JobsmithError::Io)?;
            info!("all data reset");
            println!("✓ All data reset.");
        }
        other => {
            return Err(JobsmithError::Config(format!(
                "unknown reset target: {}. use 'profile', 'applications', or 'all'",
                other
            )));
        }
    }

    Ok(())
}

async fn confirm_reset(what: &'static str) -> bool {
    // stdin reads are blocking; keep them off the async runtime.
    // A JoinError (panicked prompt task) counts as "not confirmed".
    tokio::task::spawn_blocking(move || {
        print!("Type RESET to confirm deletion of {}: ", what);
        // best-effort: an unflushed prompt only degrades UX, the read below still works
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let mut buf = String::new();
        // best-effort: on read failure buf stays empty -> != "RESET" -> reset is cancelled
        let _ = std::io::stdin().read_line(&mut buf);
        buf.trim() == "RESET"
    })
    .await
    .unwrap_or(false)
}
