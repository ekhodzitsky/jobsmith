//! Document template engine using Typst.
//!
//! Generates `.typ` source files from templates and candidate data,
//! then compiles them to PDF via `typst-cli`.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, instrument, warn};

use crate::error::{JobsmithError, Result};
use crate::hh::models::VacancyDetail;
use crate::profile::model::Profile;

/// Default timeout for Typst compilation.
const TYPST_TIMEOUT: Duration = Duration::from_secs(60);

/// Path to the CV template file at runtime.
const CV_TEMPLATE_PATH: &str = "templates/cv.typ";

/// Path to the cover letter template file at runtime.
const COVER_TEMPLATE_PATH: &str = "templates/cover.typ";

/// Embedded CV template fallback, compiled into the binary.
const CV_TEMPLATE_EMBEDDED: &str = include_str!("../../templates/cv.typ");

/// Embedded cover letter template fallback, compiled into the binary.
const COVER_TEMPLATE_EMBEDDED: &str = include_str!("../../templates/cover.typ");

/// Compile a Typst source file to PDF.
///
/// Spawns `typst compile` as a child process with a timeout.
/// Per AGENTS.md: all external `Command` calls must have timeout and `kill_on_drop`.
#[instrument(skip(input_path, output_path))]
pub async fn compile_typst(input_path: &Path, output_path: &Path) -> Result<()> {
    info!(
        input = %input_path.display(),
        output = %output_path.display(),
        "compiling typst"
    );

    let mut cmd = Command::new("typst");
    cmd.arg("compile")
        .arg(input_path)
        .arg(output_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let result =
        timeout(TYPST_TIMEOUT, cmd.output())
            .await
            .map_err(|_| JobsmithError::ProcessTimeout {
                duration_secs: TYPST_TIMEOUT.as_secs(),
            })?;

    let output = result.map_err(|e| JobsmithError::Process(format!("typst: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(%stderr, "typst compilation failed");
        return Err(JobsmithError::TemplateCompilation(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        debug!(%stdout, "typst stdout");
    }

    info!(output = %output_path.display(), "typst compilation succeeded");
    Ok(())
}

/// Generate a Typst CV source file from a profile and vacancy.
#[instrument(skip(profile, vacancy, cv_content, output_dir))]
pub async fn generate_cv_typst(
    profile: &Profile,
    vacancy: &VacancyDetail,
    cv_content: &str,
    output_dir: &Path,
) -> Result<PathBuf> {
    let file_name = format!("cv_{}.typ", sanitize_filename(vacancy.base.employer_name()));
    let output_path = output_dir.join(&file_name);

    let template = load_template(CV_TEMPLATE_PATH, CV_TEMPLATE_EMBEDDED).await?;
    let source = build_cv_typst_source(profile, vacancy, cv_content, &template);

    tokio::fs::write(&output_path, source)
        .await
        .map_err(JobsmithError::Io)?;

    debug!(path = %output_path.display(), "cv typst source written");
    Ok(output_path)
}

/// Generate a Typst cover letter source file.
#[instrument(skip(profile, vacancy, cover_content, output_dir))]
pub async fn generate_cover_typst(
    profile: &Profile,
    vacancy: &VacancyDetail,
    cover_content: &str,
    output_dir: &Path,
) -> Result<PathBuf> {
    let file_name = format!(
        "cover_{}_{}.typ",
        sanitize_filename(vacancy.base.employer_name()),
        sanitize_filename(&vacancy.base.name)
    );
    let output_path = output_dir.join(&file_name);

    let template = load_template(COVER_TEMPLATE_PATH, COVER_TEMPLATE_EMBEDDED).await?;
    let source = build_cover_typst_source(profile, vacancy, cover_content, &template);

    tokio::fs::write(&output_path, source)
        .await
        .map_err(JobsmithError::Io)?;

    debug!(path = %output_path.display(), "cover typst source written");
    Ok(output_path)
}

// ---------------------------------------------------------------------------
// Template loading
// ---------------------------------------------------------------------------

async fn load_template(path: &str, fallback: &str) -> Result<String> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => Ok(content),
        Err(e) => {
            warn!(path = path, error = %e, "template file not found, using embedded fallback");
            Ok(fallback.to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// Typst source builders
// ---------------------------------------------------------------------------

fn build_cv_typst_source(
    profile: &Profile,
    _vacancy: &VacancyDetail,
    cv_content: &str,
    template: &str,
) -> String {
    let contact = build_contact_block(profile);
    let experience = build_experience_block(profile);
    let skills = build_skills_block(profile);
    let education = build_education_block(profile);
    let certifications = build_certifications_block(profile);
    let languages = build_languages_block(profile);
    let publications = build_publications_block(profile);

    template
        .replace("{{NAME}}", &escape_typst(&profile.name))
        .replace("{{CONTACT}}", &escape_typst(&contact))
        .replace("{{SUMMARY}}", &markdown_to_typst(cv_content))
        .replace("{{EXPERIENCE}}", &experience)
        .replace("{{SKILLS}}", &skills)
        .replace("{{EDUCATION}}", &education)
        .replace("{{CERTIFICATIONS}}", &certifications)
        .replace("{{LANGUAGES}}", &languages)
        .replace("{{PUBLICATIONS}}", &publications)
}

fn build_cover_typst_source(
    profile: &Profile,
    vacancy: &VacancyDetail,
    cover_content: &str,
    template: &str,
) -> String {
    let date = chrono::Local::now().format("%d.%m.%Y").to_string();

    template
        .replace("{{DATE}}", &date)
        .replace("{{CANDIDATE_NAME}}", &escape_typst(&profile.name))
        .replace("{{CANDIDATE_CITY}}", &escape_typst(&profile.city))
        .replace("{{CANDIDATE_PHONE}}", &escape_typst(&profile.phone))
        .replace("{{CANDIDATE_EMAIL}}", &escape_typst(&profile.email))
        .replace("{{COMPANY}}", &escape_typst(vacancy.base.employer_name()))
        .replace("{{ROLE}}", &escape_typst(&vacancy.base.name))
        .replace("{{CONTENT}}", &markdown_to_typst(cover_content))
}

// ---------------------------------------------------------------------------
// Section builders
// ---------------------------------------------------------------------------

fn build_contact_block(profile: &Profile) -> String {
    let mut parts = vec![
        profile.city.clone(),
        profile.phone.clone(),
        profile.email.clone(),
    ];
    if let Some(tg) = &profile.telegram {
        parts.push(format!("Telegram: {tg}"));
    }
    if let Some(web) = &profile.website {
        parts.push(web.clone());
    }
    parts.join(" | ")
}

fn build_experience_block(profile: &Profile) -> String {
    if profile.experience.is_empty() {
        return "Нет опыта работы".to_string();
    }
    profile
        .experience
        .iter()
        .map(|e| {
            let start = e.start_date.format("%m.%Y").to_string();
            let end = e
                .end_date
                .map(|d| d.format("%m.%Y").to_string())
                .unwrap_or_else(|| "наст. время".to_string());
            let location = e.location.as_deref().unwrap_or("");
            let header = if location.is_empty() {
                format!("*{}* — {}", escape_typst(&e.role), escape_typst(&e.company))
            } else {
                format!(
                    "*{}* — {} | {}",
                    escape_typst(&e.role),
                    escape_typst(&e.company),
                    escape_typst(location)
                )
            };
            let period = format!("{start} — {end}");

            let responsibilities = if e.responsibilities.is_empty() {
                String::new()
            } else {
                e.responsibilities
                    .iter()
                    .map(|r| format!("- {}", escape_typst(r)))
                    .collect::<Vec<_>>()
                    .join("\n")
            };

            let achievements = if e.achievements.is_empty() {
                String::new()
            } else {
                e.achievements
                    .iter()
                    .map(|a| format!("- {}", escape_typst(a)))
                    .collect::<Vec<_>>()
                    .join("\n")
            };

            let tech = if e.technologies.is_empty() {
                String::new()
            } else {
                format!("Технологии: {}", escape_typst(&e.technologies.join(", ")))
            };

            let mut parts: Vec<String> = vec![header, period];
            if !responsibilities.is_empty() {
                parts.push(responsibilities);
            }
            if !achievements.is_empty() {
                parts.push(achievements);
            }
            if !tech.is_empty() {
                parts.push(tech);
            }
            parts.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn build_skills_block(profile: &Profile) -> String {
    if profile.skills.is_empty() && profile.soft_skills.is_empty() {
        return "—".to_string();
    }
    let mut parts = Vec::new();
    if !profile.skills.is_empty() {
        parts.push(format!(
            "*Технические:* {}",
            escape_typst(&profile.skills.join(", "))
        ));
    }
    if !profile.soft_skills.is_empty() {
        parts.push(format!(
            "*Soft skills:* {}",
            escape_typst(&profile.soft_skills.join(", "))
        ));
    }
    parts.join("\n\n")
}

fn build_education_block(profile: &Profile) -> String {
    if profile.education.is_empty() {
        return "—".to_string();
    }
    profile
        .education
        .iter()
        .map(|e| {
            let start = e.start_date.format("%m.%Y").to_string();
            let end = e
                .end_date
                .map(|d| d.format("%m.%Y").to_string())
                .unwrap_or_else(|| "наст. время".to_string());
            let desc = e.description.as_deref().unwrap_or("");

            let mut parts = vec![
                format!("*{}* — {}", escape_typst(&e.degree), escape_typst(&e.field)),
                format!("{} | {} — {}", escape_typst(&e.institution), start, end),
            ];
            if !desc.is_empty() {
                parts.push(escape_typst(desc));
            }
            parts.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn build_certifications_block(profile: &Profile) -> String {
    if profile.certifications.is_empty() {
        return "—".to_string();
    }
    profile
        .certifications
        .iter()
        .map(|c| {
            let date = c
                .date
                .map(|d| d.format("%m.%Y").to_string())
                .unwrap_or_default();
            let date_str = if date.is_empty() {
                String::new()
            } else {
                format!(" | {date}")
            };
            let hours = c.hours.map(|h| format!(" | {h} ч.")).unwrap_or_default();
            format!(
                "*{}* — {}{}{}",
                escape_typst(&c.name),
                escape_typst(&c.issuer),
                date_str,
                hours
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_languages_block(profile: &Profile) -> String {
    if profile.languages.is_empty() {
        return "—".to_string();
    }
    profile
        .languages
        .iter()
        .map(|l| format!("*{}* — {:?}", escape_typst(&l.name), l.level))
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_publications_block(profile: &Profile) -> String {
    if profile.publications.is_empty() {
        return "—".to_string();
    }
    profile
        .publications
        .iter()
        .map(|p| format!("- {}", escape_typst(p)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn escape_typst(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '#' | '*' | '_' | '`' | '$' | '@' | '~' | '^' | '&' | '<' | '>' | '"' | '[' | ']'
            | '{' | '}' => {
                result.push('\\');
                result.push(c);
            }
            _ => result.push(c),
        }
    }
    result
}

/// Convert Markdown to Typst markup.
///
/// Typst and Markdown share `*bold*`, `_italic_`, and `- lists`,
/// but headings use `# ` in Markdown and `= ` in Typst.
///
/// The input is AI output derived from externally controlled vacancy
/// descriptions, so any `#` outside the heading conversion is escaped:
/// `#` is the only character that switches Typst markup into code mode
/// (`#read(...)`, `#image(...)`).
fn markdown_to_typst(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("### ") {
            result.push_str("=== ");
            result.push_str(&escape_typst_code(rest));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            result.push_str("== ");
            result.push_str(&escape_typst_code(rest));
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            result.push_str("= ");
            result.push_str(&escape_typst_code(rest));
        } else {
            result.push_str(&escape_typst_code(line));
        }
        result.push('\n');
    }
    result
}

/// Escape `#` so untrusted content cannot invoke Typst code.
///
/// Unlike [`escape_typst`], shared Markdown/Typst formatting
/// (`*bold*`, `_italic_`, `- lists`) is intentionally left intact.
fn escape_typst_code(text: &str) -> String {
    text.replace('#', "\\#")
}

pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_lowercase()
}

/// Create the output directory for documents if it doesn't exist.
pub async fn ensure_output_dir(base_dir: &Path) -> Result<PathBuf> {
    let dir = base_dir.join("output");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(JobsmithError::Io)?;
    Ok(dir)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_typst_escapes_special_chars() {
        let input = "#hello *world* `code` $math$ [block]";
        let expected = r"\#hello \*world\* \`code\` \$math\$ \[block\]";
        assert_eq!(escape_typst(input), expected);
    }

    #[test]
    fn markdown_to_typst_escapes_typst_code_injection() {
        let input = "#read(\"x\")\nsee #image(\"y\") inline\n## H";
        let expected = "\\#read(\"x\")\nsee \\#image(\"y\") inline\n== H\n";
        assert_eq!(markdown_to_typst(input), expected);
    }

    #[test]
    fn markdown_to_typst_escapes_hash_in_heading_text() {
        let input = "## Опыт #eval(\"1+1\")";
        let expected = "== Опыт \\#eval(\"1+1\")\n";
        assert_eq!(markdown_to_typst(input), expected);
    }

    #[test]
    fn markdown_to_typst_converts_headings() {
        let input = "# Heading 1\n## Heading 2\n### Heading 3\nRegular line with *bold* and _italic_\n- list item";
        let expected = "= Heading 1\n== Heading 2\n=== Heading 3\nRegular line with *bold* and _italic_\n- list item\n";
        assert_eq!(markdown_to_typst(input), expected);
    }

    #[test]
    fn sanitize_filename_replaces_invalid_chars() {
        assert_eq!(sanitize_filename("Hello World!"), "hello_world_");
        assert_eq!(sanitize_filename("foo-bar_baz"), "foo-bar_baz");
    }

    #[tokio::test]
    async fn ensure_output_dir_creates_exactly_base_output() {
        let base = std::env::temp_dir().join(format!("jobsmith-outdir-{}", std::process::id()));
        let dir = ensure_output_dir(&base).await.unwrap();
        assert_eq!(dir, base.join("output"));
        assert!(dir.is_dir());
        assert!(
            !dir.join("output").exists(),
            "must not create a nested output/output"
        );
        // best-effort cleanup of the temp dir
        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[test]
    fn build_contact_block_includes_optional_fields() {
        let profile = Profile {
            city: "Москва".to_string(),
            phone: "+7 999 123-45-67".to_string(),
            email: "test@example.com".to_string(),
            telegram: Some("@testuser".to_string()),
            website: Some("https://example.com".to_string()),
            ..Default::default()
        };

        let contact = build_contact_block(&profile);
        assert!(contact.contains("Москва"));
        assert!(contact.contains("Telegram: @testuser"));
        assert!(contact.contains("https://example.com"));
    }

    #[test]
    fn build_skills_block_shows_both_categories() {
        let profile = Profile {
            skills: vec!["Rust".to_string(), "Python".to_string()],
            soft_skills: vec!["Коммуникабельность".to_string()],
            ..Default::default()
        };

        let skills = build_skills_block(&profile);
        assert!(skills.contains("Технические:"));
        assert!(skills.contains("Soft skills:"));
    }

    #[test]
    fn build_languages_block_formats_correctly() {
        use crate::profile::model::{Language, LanguageLevel};

        let mut profile = Profile::default();
        profile.languages.push(Language {
            name: "Английский".to_string(),
            level: LanguageLevel::C1,
        });

        let languages = build_languages_block(&profile);
        assert!(languages.contains("Английский"));
        assert!(languages.contains("C1"));
    }
}
