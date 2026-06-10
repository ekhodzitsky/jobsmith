//! CLI argument parsing using clap.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// AI-powered job application assistant for HeadHunter (Russia).
#[derive(Debug, Parser)]
#[command(name = "jobsmith")]
#[command(about = "AI-powered job application assistant for HeadHunter")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    /// Path to the data directory.
    #[arg(short, long, value_name = "DIR")]
    pub data_dir: Option<PathBuf>,

    /// Enable verbose logging.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Subcommand.
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Set up or update your candidate profile.
    Setup {
        /// Update only a specific section.
        #[arg(short, long)]
        section: Option<String>,
    },

    /// Search for vacancies on HeadHunter.
    Search {
        /// Search query text.
        #[arg(value_name = "QUERY")]
        text: Option<String>,

        /// City / area ID.
        #[arg(short, long)]
        area: Option<i32>,

        /// Experience level (noExperience, between1And3, between3And6, moreThan6).
        #[arg(short, long)]
        experience: Option<String>,

        /// Employment type (full, part, project, volunteer, probation).
        // long-only: -e belongs to --experience
        #[arg(long)]
        employment: Option<String>,

        /// Schedule (fullDay, shift, flexible, remote, flyInFlyOut).
        // long-only: -s belongs to --salary
        #[arg(long)]
        schedule: Option<String>,

        /// Minimum salary.
        #[arg(short, long)]
        salary: Option<i32>,

        /// Only show vacancies with salary.
        #[arg(long)]
        with_salary: bool,

        /// Number of results per page.
        #[arg(short, long, default_value = "20")]
        per_page: i32,

        /// Result page (1-based).
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(i32).range(1..))]
        page: i32,

        /// Launch interactive TUI browser.
        #[arg(short = 'i', long)]
        interactive: bool,
    },

    /// Apply to a vacancy (run the full AI workflow).
    Apply {
        /// Vacancy URL or ID.
        #[arg(value_name = "URL_OR_ID")]
        vacancy: String,

        /// Skip fit evaluation and proceed directly.
        #[arg(short, long)]
        force: bool,
    },

    /// List tracked applications.
    List {
        /// Show all details.
        #[arg(short, long)]
        detailed: bool,
    },

    /// Mark a tracked application as submitted.
    MarkApplied {
        /// Application ID from `jobsmith list`.
        #[arg(value_name = "ID")]
        id: i64,
    },

    /// Look up salary benchmarks.
    Salary {
        /// Company name to search.
        #[arg(value_name = "COMPANY")]
        company: String,

        /// Filter by city.
        #[arg(short, long)]
        city: Option<String>,

        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Reset profile or application data.
    Reset {
        /// What to reset (profile, applications, all).
        #[arg(value_name = "TARGET")]
        target: String,
    },
}
