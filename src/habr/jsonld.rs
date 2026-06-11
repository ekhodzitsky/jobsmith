//! Extractor for the schema.org `JobPosting` JSON-LD on Habr Career
//! vacancy pages.

use scraper::{Html, Selector};
use serde_json::Value;

use crate::error::{JobsmithError, Result};
use crate::hh::models::{Area, Employer, Vacancy, VacancyDetail};

const HABR_VACANCY_BASE: &str = "https://career.habr.com/vacancies";

/// Parse a Habr Career vacancy page into a [`VacancyDetail`].
///
/// Reads the page's schema.org `JobPosting` JSON-LD block (the same
/// structured data Habr publishes for search engines) rather than
/// scraping the rendered DOM, so it survives layout changes.
pub fn parse_vacancy_html(html: &str, id: &str) -> Result<VacancyDetail> {
    let posting = find_job_posting(html)
        .ok_or_else(|| JobsmithError::ResponseParse("habr: no JobPosting JSON-LD".to_string()))?;

    let name = str_field(&posting, "title")
        .ok_or_else(|| JobsmithError::ResponseParse("habr: JobPosting has no title".to_string()))?;
    let description = str_field(&posting, "description");
    let employer = employer_from(&posting);
    let area = area_from(&posting);

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

/// Find the `JobPosting` object among the page's JSON-LD blocks.
///
/// Handles both a bare object and a `@graph` array wrapper.
fn find_job_posting(html: &str) -> Option<Value> {
    let doc = Html::parse_document(html);
    let selector = Selector::parse(r#"script[type="application/ld+json"]"#).ok()?;
    for script in doc.select(&selector) {
        let raw = script.text().collect::<String>();
        let Ok(value) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        if let Some(found) = job_posting_in(&value) {
            return Some(found);
        }
    }
    None
}

fn job_posting_in(value: &Value) -> Option<Value> {
    if value.get("@type").and_then(Value::as_str) == Some("JobPosting") {
        return Some(value.clone());
    }
    if let Some(graph) = value.get("@graph").and_then(Value::as_array) {
        return graph.iter().find_map(job_posting_in);
    }
    if let Some(items) = value.as_array() {
        return items.iter().find_map(job_posting_in);
    }
    None
}

fn str_field(posting: &Value, key: &str) -> Option<String> {
    posting
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

fn employer_from(posting: &Value) -> Option<Employer> {
    let org = posting.get("hiringOrganization")?;
    let name = org.get("name").and_then(Value::as_str).map(str::to_string);
    let url = org
        .get("sameAs")
        .and_then(Value::as_str)
        .map(str::to_string);
    name.as_ref()?;
    Some(Employer {
        id: None,
        name,
        url,
        ..Default::default()
    })
}

fn area_from(posting: &Value) -> Option<Area> {
    let loc = posting.get("jobLocation")?;
    // jobLocation may be an array or a single Place object.
    let place = loc.as_array().and_then(|a| a.first()).unwrap_or(loc);
    let address = place.get("address")?;
    // address may be a plain string or a PostalAddress object.
    let name = address
        .as_str()
        .map(str::to_string)
        .or_else(|| {
            address
                .get("addressLocality")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .filter(|s| !s.is_empty())?;
    Some(Area {
        id: None,
        name: Some(name),
        url: None,
        parent_id: None,
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
