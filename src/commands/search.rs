//! Search command — queries HeadHunter API and displays results.

use std::path::Path;

use tracing::{info, instrument};

use crate::error::{JobsmithError, Result};
use crate::hh::client::HhClient;
use crate::hh::models::{strip_html, VacancySearchQuery};
use crate::profile::store::ProfileStore;
use crate::tui;

/// Run the search command.
#[instrument(skip(query))]
pub async fn run(
    query: VacancySearchQuery,
    interactive: bool,
    data_dir: Option<&Path>,
) -> Result<()> {
    let client = HhClient::new()?;

    info!("searching hh.ru");
    let response = client.search_vacancies(&query).await?;

    if interactive {
        let mut app = tui::App::new(response.items)?;
        match app.run().await? {
            tui::Action::Apply(id) => {
                let db_path = data_dir
                    .ok_or_else(|| {
                        JobsmithError::Config("data directory required for apply".to_string())
                    })?
                    .join("jobsmith.db");
                let store = ProfileStore::open(&db_path).await?;
                crate::commands::apply::run(&store, &id, false).await
            }
            tui::Action::Quit => Ok(()),
        }
    } else {
        println!(
            "\nFound {} vacancies (page {}/{}):\n",
            response.found,
            query.page + 1,
            response.pages
        );

        for vacancy in &response.items {
            let salary = vacancy
                .salary
                .as_ref()
                .map(|s| {
                    let from = s
                        .from
                        .map(|v| format!("{v}"))
                        .unwrap_or_else(|| "?".to_string());
                    let to =
                        s.to.map(|v| format!("{v}"))
                            .unwrap_or_else(|| "?".to_string());
                    let currency = s.currency.as_deref().unwrap_or("RUR");
                    format!("{} — {} {}", from, to, currency)
                })
                .unwrap_or_else(|| "з/п не указана".to_string());

            let area = vacancy
                .area
                .as_ref()
                .and_then(|a| a.name.clone())
                .unwrap_or_else(|| "?".to_string());

            let experience = vacancy
                .experience
                .as_ref()
                .and_then(|e| e.name.clone())
                .unwrap_or_else(|| "?".to_string());

            let snippet = vacancy
                .snippet
                .as_ref()
                .and_then(|s| s.requirement.as_ref().or(s.responsibility.as_ref()))
                .map(|s| strip_html(s))
                .unwrap_or_default();

            println!("{}", "─".repeat(60));
            println!("{} | {}", vacancy.id, vacancy.name);
            println!("  {} | {} | {}", vacancy.employer_name(), area, salary);
            println!("  Опыт: {}", experience);
            if !snippet.is_empty() {
                println!("  {}", truncate(&snippet, 150));
            }
            println!();
        }

        println!("{}", "─".repeat(60));
        println!("Use `jobsmith apply <id>` to apply to a vacancy.");

        Ok(())
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    let count = s.chars().count();
    if count <= max_len {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max_len).collect::<String>())
    }
}
