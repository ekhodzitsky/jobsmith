//! Jobsmith — AI-powered job application assistant for HeadHunter (Russia).

use std::path::PathBuf;
use std::sync::OnceLock;

use clap::Parser;
use tracing::{error, info};

use jobsmith::cli::{Cli, Commands};
use jobsmith::commands::{apply, list, reset, salary_cmd, search, setup};
use jobsmith::hh::models::VacancySearchQuery;
use jobsmith::profile::store::ProfileStore;

#[tokio::main]
async fn main() {
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
        std::process::exit(1);
    }

    let db_path = data_dir.join("jobsmith.db");

    let result = match cli.command {
        Commands::Setup { section } => {
            let store = match ProfileStore::open(&db_path).await {
                Ok(s) => s,
                Err(e) => {
                    error!(error = %e, "failed to open profile database");
                    std::process::exit(1);
                }
            };
            setup::run(&store, section.as_deref()).await
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
                page: 0,
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
                    std::process::exit(1);
                }
            };
            apply::run(&store, &vacancy, force).await
        }
        Commands::List { detailed } => {
            let store = match ProfileStore::open(&db_path).await {
                Ok(s) => s,
                Err(e) => {
                    error!(error = %e, "failed to open profile database");
                    std::process::exit(1);
                }
            };
            list::run(&store, detailed).await
        }
        Commands::Reset { target } => reset::run(&target, &data_dir).await,
        Commands::Salary {
            company,
            city,
            json,
        } => salary_cmd::run(&company, city.as_deref(), json, &data_dir),
    };

    if let Err(e) = result {
        error!(error = %e, "command failed");
        std::process::exit(1);
    }

    info!("jobsmith finished successfully");
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
