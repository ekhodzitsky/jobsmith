//! Search command — queries a job board and displays results.

use std::path::Path;

use tracing::{info, instrument};

use crate::cli::VacancySource;
use crate::error::{JobsmithError, Result};
use crate::habr::HabrClient;
use crate::hh::client::HhClient;
use crate::hh::models::{strip_html, Vacancy, VacancySearchQuery};
use crate::profile::store::ProfileStore;
use crate::tui;

/// Run the search command.
#[instrument(skip(query))]
pub async fn run(
    query: VacancySearchQuery,
    interactive: bool,
    data_dir: Option<&Path>,
    source: VacancySource,
) -> Result<()> {
    let (items, header) = match source {
        VacancySource::Hh => {
            info!("searching hh.ru");
            let response = HhClient::new()?.search_vacancies(&query).await?;
            let header = format!(
                "\nFound {} vacancies (page {}/{}):\n",
                response.found,
                query.page + 1,
                response.pages
            );
            (response.items, header)
        }
        VacancySource::Habr => {
            let text = query.text.as_deref().unwrap_or_default();
            if text.is_empty() {
                return Err(JobsmithError::Config(
                    "habr search requires a query, e.g. `jobsmith search rust --source habr`"
                        .to_string(),
                ));
            }
            info!("searching career.habr.com");
            let items = HabrClient::new()?.search(text).await?;
            let header = format!("\nFound {} vacancies on Habr Career:\n", items.len());
            (items, header)
        }
    };

    if interactive {
        // The TUI detail view loads from the HH API; gate it to HH for now.
        if source != VacancySource::Hh {
            return Err(JobsmithError::Config(
                "interactive mode is HeadHunter-only for now; run without --interactive"
                    .to_string(),
            ));
        }
        let mut app = tui::App::new(items)?;
        match app.run().await? {
            tui::Action::Apply(id) => {
                let data_dir = data_dir.ok_or_else(|| {
                    JobsmithError::Config("data directory required for apply".to_string())
                })?;
                let store = ProfileStore::open(&data_dir.join("jobsmith.db")).await?;
                crate::commands::apply::run(&store, &id, false, data_dir, source).await
            }
            tui::Action::Quit => Ok(()),
        }
    } else {
        print!("{header}");
        print_vacancies(&items);
        println!("{}", "─".repeat(60));
        println!("Use `jobsmith apply <id|url>` to apply to a vacancy.");
        Ok(())
    }
}

fn print_vacancies(items: &[Vacancy]) {
    for vacancy in items {
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

        let experience = vacancy.experience.as_ref().and_then(|e| e.name.clone());

        let snippet = vacancy
            .snippet
            .as_ref()
            .and_then(|s| s.requirement.as_ref().or(s.responsibility.as_ref()))
            .map(|s| strip_html(s))
            .unwrap_or_default();

        println!("{}", "─".repeat(60));
        println!("{} | {}", vacancy.id, vacancy.name);
        println!("  {} | {} | {}", vacancy.employer_name(), area, salary);
        if let Some(exp) = experience {
            println!("  Опыт: {}", exp);
        }
        if !snippet.is_empty() {
            println!("  {}", truncate(&snippet, 150));
        }
        println!();
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
