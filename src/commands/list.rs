//! List tracked applications.

use crate::error::Result;
use crate::profile::store::ProfileStore;

/// Run the list command.
pub fn run(store: &ProfileStore, detailed: bool) -> Result<()> {
    let apps = store.list_applications()?;

    if apps.is_empty() {
        println!("No applications tracked yet.");
        return Ok(());
    }

    println!("\n=== Applications ===\n");

    for app in &apps {
        println!("{}", "─".repeat(60));
        println!("ID: {} | Status: {}", app.id, app.status);
        println!(
            "Vacancy: {} | {}",
            app.vacancy_id,
            app.vacancy_name.as_deref().unwrap_or("unknown")
        );
        if let Some(employer) = &app.employer_name {
            println!("Employer: {}", employer);
        }
        if let Some(score) = app.fit_score {
            println!("Fit score: {}/100", score);
        }
        if detailed {
            if let Some(cv) = &app.cv_path {
                println!("CV: {}", cv);
            }
            if let Some(cover) = &app.cover_path {
                println!("Cover: {}", cover);
            }
            if let Some(notes) = &app.notes {
                println!("Notes: {}", notes);
            }
            println!("Created: {} | Updated: {}", app.created_at, app.updated_at);
        }
        println!();
    }

    println!("{}", "─".repeat(60));
    println!("Total: {} applications", apps.len());

    Ok(())
}
