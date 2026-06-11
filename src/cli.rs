//! CLI argument parsing using clap.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Which job board to query.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum VacancySource {
    /// HeadHunter (`hh.ru`) — requires an OAuth token for live access.
    #[default]
    Hh,
    /// Habr Career (`career.habr.com`) — public RSS + JSON-LD, no token.
    Habr,
}

impl VacancySource {
    /// Infer the source from a vacancy URL or id.
    ///
    /// A Habr Career URL is recognized by its host; everything else
    /// (an hh.ru URL or a bare numeric id) defaults to HeadHunter.
    pub fn detect(input: &str) -> Self {
        if input.contains("career.habr.com") {
            Self::Habr
        } else {
            Self::Hh
        }
    }
}

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
    Setup,

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

        /// Result page (1-based, HeadHunter only).
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(i32).range(1..))]
        page: i32,

        /// Job board to search.
        #[arg(long, value_enum, default_value_t = VacancySource::Hh)]
        source: VacancySource,

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
        /// Company name to search in the local salary_data.json.
        #[arg(
            value_name = "COMPANY",
            required_unless_present = "role",
            conflicts_with = "role"
        )]
        company: Option<String>,

        /// Filter by city (local lookup only).
        #[arg(short, long, conflicts_with = "role")]
        city: Option<String>,

        /// HH professional role ID for online statistics (e.g. 96 = developer).
        #[arg(long, requires = "area")]
        role: Option<String>,

        /// HH area ID for online statistics (e.g. 1 = Moscow).
        #[arg(long, requires = "role")]
        area: Option<String>,

        /// Currency for online statistics.
        #[arg(long, default_value = "RUR")]
        currency: String,

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_recognizes_habr_url() {
        assert_eq!(
            VacancySource::detect("https://career.habr.com/vacancies/1000166679"),
            VacancySource::Habr
        );
    }

    #[test]
    fn detect_defaults_to_hh_for_hh_url_and_bare_id() {
        assert_eq!(
            VacancySource::detect("https://hh.ru/vacancy/123456"),
            VacancySource::Hh
        );
        assert_eq!(VacancySource::detect("123456"), VacancySource::Hh);
    }
}
