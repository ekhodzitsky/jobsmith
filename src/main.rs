//! Jobsmith — AI-powered job application assistant for HeadHunter (Russia).

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::OnceLock;

use clap::Parser;
use tracing::{error, info};

use jobsmith::cli::{Cli, Commands};
use jobsmith::commands::{apply, list, mark_applied, reset, salary_cmd, search, setup};
use jobsmith::error::JobsmithError;
use jobsmith::hh::models::VacancySearchQuery;
use jobsmith::profile::store::ProfileStore;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => {
            info!("jobsmith finished successfully");
            ExitCode::SUCCESS
        }
        Err(e) => {
            error!(error = %e, "command failed");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), JobsmithError> {
    let cli = Cli::parse();

    init_tracing(cli.verbose);

    info!("jobsmith starting");

    let data_dir = cli.data_dir.unwrap_or_else(|| {
        dirs::data_dir()
            .map(|d| d.join("jobsmith"))
            .unwrap_or_else(|| PathBuf::from("./jobsmith-data"))
    });

    // Ensure data directory exists
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        error!(error = %e, "failed to create data directory");
        return Err(JobsmithError::Io(e));
    }
    // The directory holds the PII database and generated documents.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(&data_dir, std::fs::Permissions::from_mode(0o700))
        {
            error!(error = %e, "failed to restrict data directory permissions");
            return Err(JobsmithError::Io(e));
        }
    }

    let db_path = data_dir.join("jobsmith.db");

    match cli.command {
        Commands::Setup => {
            let store = match ProfileStore::open(&db_path).await {
                Ok(s) => s,
                Err(e) => {
                    error!(error = %e, "failed to open profile database");
                    return Err(e);
                }
            };
            setup::run(&store).await
        }
        Commands::Search {
            text,
            area,
            experience,
            employment,
            schedule,
            salary,
            with_salary,
            per_page,
            page,
            interactive,
        } => {
            let query = VacancySearchQuery {
                text,
                area,
                experience,
                employment,
                schedule,
                salary,
                currency: Some("RUR".to_string()),
                only_with_salary: with_salary,
                // CLI is 1-based, HH API is 0-based
                page: page - 1,
                per_page,
                order_by: None,
                search_field: None,
                professional_role: None,
            };
            search::run(query, interactive, Some(&data_dir)).await
        }
        Commands::Apply { vacancy, force } => {
            let store = match ProfileStore::open(&db_path).await {
                Ok(s) => s,
                Err(e) => {
                    error!(error = %e, "failed to open profile database");
                    return Err(e);
                }
            };
            apply::run(&store, &vacancy, force, &data_dir).await
        }
        Commands::List { detailed } => {
            let store = match ProfileStore::open(&db_path).await {
                Ok(s) => s,
                Err(e) => {
                    error!(error = %e, "failed to open profile database");
                    return Err(e);
                }
            };
            list::run(&store, detailed).await
        }
        Commands::MarkApplied { id } => {
            let store = match ProfileStore::open(&db_path).await {
                Ok(s) => s,
                Err(e) => {
                    error!(error = %e, "failed to open profile database");
                    return Err(e);
                }
            };
            mark_applied::run(&store, id).await
        }
        Commands::Reset { target } => reset::run(&target, &data_dir).await,
        Commands::Salary {
            company,
            city,
            json,
        } => salary_cmd::run(&company, city.as_deref(), json, &data_dir),
    }
}

fn init_tracing(verbose: u8) {
    let filter = match verbose {
        0 => "jobsmith=warn",
        1 => "jobsmith=info",
        2 => "jobsmith=debug",
        _ => "jobsmith=trace",
    };

    static TRACING_INIT: OnceLock<()> = OnceLock::new();
    TRACING_INIT.get_or_init(|| {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_level(true)
            .init();
    });
}
