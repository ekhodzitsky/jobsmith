//! Interactive profile setup command.
//!
//! Guides the user through entering their profile information.

use std::io::{self, Write};

use tracing::{info, instrument};

use crate::error::{JobsmithError, Result};
use crate::profile::model::{
    Education, Experience, Language, LanguageLevel, Profile, WorkPermit,
};
use crate::profile::store::ProfileStore;
use chrono::NaiveDate;

/// Run the profile setup wizard.
#[instrument(skip(store))]
pub async fn run(store: &ProfileStore, section: Option<&str>) -> Result<()> {
    if let Some(sec) = section {
        info!(section = %sec, "updating profile section");
        todo!("section-specific update not yet implemented");
    }

    println!("\n=== Jobsmith Profile Setup ===\n");
    println!("Enter your profile information. Press Enter to skip optional fields.\n");

    let name = read_line("Full name: ")?;
    let city = read_line("City: ")?;
    let phone = read_line("Phone (+7...): ")?;
    let email = read_line("Email: ")?;
    let telegram = read_optional("Telegram (optional): ")?;
    let website = read_optional("Website / LinkedIn (optional): ")?;
    let citizenship = read_line("Citizenship: ")?;
    let work_permit = match read_line("Work permit (citizen/permanent/temporary/visa/none): ")?.as_str() {
        "permanent" | "permanent_resident" => WorkPermit::PermanentResident,
        "temporary" | "temporary_resident" => WorkPermit::TemporaryResident,
        "visa" | "work_visa" => WorkPermit::WorkVisa,
        "none" | "no_permit" => WorkPermit::NoPermit,
        _ => WorkPermit::Citizen,
    };
    let ready_to_relocate = read_bool("Ready to relocate? (y/n): ")?;
    let work_format = read_optional("Preferred work format (remote/office/hybrid): ")?;

    // Languages
    let mut languages = Vec::new();
    while let Some(lang_name) = read_optional("Language (empty to finish): ")? {
        let level = match read_line("Level (native/c2/c1/b2/b1/a2/a1): ")?.as_str() {
            "native" => LanguageLevel::Native,
            "c2" => LanguageLevel::C2,
            "c1" => LanguageLevel::C1,
            "b2" => LanguageLevel::B2,
            "b1" => LanguageLevel::B1,
            "a2" => LanguageLevel::A2,
            _ => LanguageLevel::A1,
        };
        languages.push(Language { name: lang_name, level });
    }

    // Education
    let mut education = Vec::new();
    while let Some(institution) = read_optional("Institution (empty to finish): ")? {
        education.push(Education {
            institution,
            degree: read_line("Degree: ")?,
            field: read_line("Field: ")?,
            start_date: parse_date(&read_line("Start date (YYYY-MM-DD): ")?)?,
            end_date: parse_optional_date("End date (YYYY-MM-DD, empty if current): ")?,
            description: read_optional("Description: ")?,
        });
    }

    // Experience
    let mut experience = Vec::new();
    while let Some(company) = read_optional("Company (empty to finish): ")? {
        let current = read_bool("Current job? (y/n): ")?;
        experience.push(Experience {
            company,
            role: read_line("Role: ")?,
            location: read_optional("Location: ")?,
            start_date: parse_date(&read_line("Start date (YYYY-MM-DD): ")?)?,
            end_date: if current {
                None
            } else {
                parse_optional_date("End date (YYYY-MM-DD): ")?
            },
            current,
            responsibilities: read_list("Responsibilities (empty line to finish):")?,
            achievements: read_list("Achievements (empty line to finish):")?,
            technologies: read_list("Technologies used (empty line to finish):")?,
        });
    }

    // Skills
    let skills = read_list("Technical skills (empty line to finish):")?;
    let soft_skills = read_list("Soft skills (empty line to finish):")?;

    // Summary
    println!("\nProfessional summary (3-5 sentences):");
    let summary = read_multiline()?;

    // Target roles
    let target_roles = read_list("Target roles (empty line to finish):")?;

    // Target salary
    let (target_salary, target_currency) = if let Some(s) = read_optional("Target salary (RUB, optional): ")? {
        (s.parse().ok(), Some("RUR".to_string()))
    } else {
        (None, None)
    };

    // Deal breakers
    let deal_breakers = read_list("Deal breakers (empty line to finish):")?;

    // Motivations
    let motivations = read_list("What excites you professionally (empty line to finish):")?;

    let ideal_environment = read_line("Describe your ideal work environment: ")?;

    let profile = Profile {
        name,
        city,
        phone,
        email,
        telegram,
        website,
        citizenship,
        work_permit,
        ready_to_relocate,
        work_format,
        languages,
        education,
        experience,
        skills,
        soft_skills,
        summary,
        target_roles,
        target_salary,
        target_currency,
        deal_breakers,
        motivations,
        ideal_environment,
        ..Default::default()
    };

    profile.validate()?;
    store.save_profile(&profile).await?;

    println!("\n✓ Profile saved successfully.");
    Ok(())
}

fn read_line(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout()
        .flush()
        .map_err(JobsmithError::Io)?;
    let mut buf = String::new();
    let bytes = io::stdin()
        .read_line(&mut buf)
        .map_err(JobsmithError::Io)?;
    if bytes == 0 {
        return Err(JobsmithError::Cancelled("EOF on stdin".to_string()));
    }
    Ok(buf.trim().to_string())
}

fn read_optional(prompt: &str) -> Result<Option<String>> {
    let s = read_line(prompt)?;
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(s))
    }
}

fn read_bool(prompt: &str) -> Result<bool> {
    let s = read_line(prompt)?.to_lowercase();
    Ok(s == "y" || s == "yes" || s == "да" || s == "д")
}

fn read_list(prompt: &str) -> Result<Vec<String>> {
    println!("{}", prompt);
    let mut items = Vec::new();
    loop {
        let item = read_line("- ")?;
        if item.is_empty() {
            break;
        }
        items.push(item);
    }
    Ok(items)
}

fn read_multiline() -> Result<String> {
    let mut lines = Vec::new();
    loop {
        let line = read_line("")?;
        if line.is_empty() && !lines.is_empty() {
            break;
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    Ok(lines.join("\n"))
}

fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| JobsmithError::Config(format!("invalid date: {e}")))
}

fn parse_optional_date(prompt: &str) -> Result<Option<NaiveDate>> {
    let s = read_line(prompt)?;
    if s.is_empty() {
        Ok(None)
    } else {
        NaiveDate::parse_from_str(&s, "%Y-%m-%d")
            .map(Some)
            .map_err(|e| JobsmithError::Config(format!("invalid date: {e}")))
    }
}
