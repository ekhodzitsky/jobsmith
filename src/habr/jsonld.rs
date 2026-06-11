//! Extractor for the schema.org `JobPosting` JSON-LD on Habr Career
//! vacancy pages.

use crate::error::{JobsmithError, Result};
use crate::hh::models::{Vacancy, VacancyDetail};
use crate::jsonld;

const HABR_VACANCY_BASE: &str = "https://career.habr.com/vacancies";

/// Parse a Habr Career vacancy page into a [`VacancyDetail`].
///
/// Reads the page's schema.org `JobPosting` JSON-LD block (the same
/// structured data Habr publishes for search engines) rather than
/// scraping the rendered DOM, so it survives layout changes.
pub fn parse_vacancy_html(html: &str, id: &str) -> Result<VacancyDetail> {
    let posting = jsonld::find_job_posting(html)
        .ok_or_else(|| JobsmithError::ResponseParse("habr: no JobPosting JSON-LD".to_string()))?;

    let name = jsonld::str_field(&posting, "title")
        .ok_or_else(|| JobsmithError::ResponseParse("habr: JobPosting has no title".to_string()))?;
    let description = jsonld::str_field(&posting, "description");
    let employer = jsonld::employer_from(&posting);
    let area = jsonld::area_from(&posting);

    Ok(VacancyDetail {
        base: Vacancy {
            id: id.to_string(),
            name,
            description,
            employer,
            area,
            alternate_url: Some(format!("{HABR_VACANCY_BASE}/{id}")),
            ..Default::default()
        },
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = include_str!("../../tests/fixtures/habr_vacancy.html");

    #[test]
    fn extracts_core_fields() {
        let detail = parse_vacancy_html(PAGE, "1000166679").unwrap();
        assert_eq!(detail.base.name, "Разработчик Rust");
        assert_eq!(detail.employer_name(), "Bell Integrator");
        assert_eq!(
            detail.base.area.as_ref().and_then(|a| a.name.as_deref()),
            Some("Москва")
        );
        assert_eq!(
            detail.base.alternate_url.as_deref(),
            Some("https://career.habr.com/vacancies/1000166679")
        );
    }

    #[test]
    fn description_carries_html_for_the_ai_pipeline() {
        let detail = parse_vacancy_html(PAGE, "1000166679").unwrap();
        let desc = detail.base.description.expect("description present");
        assert!(desc.contains("Rust"), "desc head: {:.80}", desc);
        assert!(desc.contains("Bell Integrator"));
    }

    #[test]
    fn missing_job_posting_errors() {
        let html = "<html><head></head><body>no structured data</body></html>";
        let err = parse_vacancy_html(html, "1").unwrap_err();
        assert!(matches!(err, JobsmithError::ResponseParse(_)), "{err:?}");
    }

    #[test]
    fn handles_postal_address_object() {
        let html = r#"<html><head><script type="application/ld+json">
            {"@context":"https://schema.org/","@type":"JobPosting","title":"Dev",
             "jobLocation":{"@type":"Place","address":{"@type":"PostalAddress","addressLocality":"Казань"}}}
            </script></head><body></body></html>"#;
        let detail = parse_vacancy_html(html, "5").unwrap();
        assert_eq!(
            detail.base.area.as_ref().and_then(|a| a.name.as_deref()),
            Some("Казань")
        );
    }
}
