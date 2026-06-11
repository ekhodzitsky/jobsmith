//! Parser for the GeekJob search listing page.

use scraper::{Html, Selector};

use crate::error::{JobsmithError, Result};
use crate::hh::models::{Employer, Vacancy};

const GEEKJOB_VACANCY_BASE: &str = "https://geekjob.ru/vacancy";

/// Parse the search page (`https://geekjob.ru/?qs=...`) into listings.
///
/// Cards are `li.collection-item` elements; ad cards link elsewhere and
/// are skipped by requiring an `/vacancy/{hex}` href.
pub fn parse_search_html(html: &str) -> Result<Vec<Vacancy>> {
    let doc = Html::parse_document(html);
    let card_sel = selector("li.collection-item")?;
    let link_sel = selector(r#"a[href^="/vacancy/"]"#)?;
    let title_sel = selector("p.truncate")?;
    let company_sel = selector("p.truncate.company-name")?;

    let mut seen = std::collections::HashSet::new();
    let mut vacancies = Vec::new();

    for card in doc.select(&card_sel) {
        let Some(href) = card
            .select(&link_sel)
            .next()
            .and_then(|a| a.value().attr("href"))
        else {
            continue; // ad / non-vacancy card
        };
        let Some(id) = hex_id_from_path(href) else {
            continue;
        };
        if !seen.insert(id.clone()) {
            continue;
        }

        let company = card.select(&company_sel).next().map(|el| collect_text(&el));
        // The first p.truncate is the role title; company-name also
        // matches p.truncate, so skip the one equal to the company.
        let name = card
            .select(&title_sel)
            .map(|el| collect_text(&el))
            .find(|t| !t.is_empty() && Some(t) != company.as_ref())
            .unwrap_or_default();
        if name.is_empty() {
            continue;
        }

        let employer = company.filter(|c| !c.is_empty()).map(|name| Employer {
            id: None,
            name: Some(name),
            ..Default::default()
        });

        vacancies.push(Vacancy {
            id: id.clone(),
            name,
            employer,
            alternate_url: Some(format!("{GEEKJOB_VACANCY_BASE}/{id}")),
            ..Default::default()
        });
    }

    Ok(vacancies)
}

/// Extract a GeekJob hex vacancy id from a URL or a bare id.
pub fn extract_hex_id(input: &str) -> Result<String> {
    let candidate = input
        .split_once("/vacancy/")
        .map_or(input, |(_, tail)| tail);
    let candidate = candidate
        .trim_matches('/')
        .split(['?', '#', '/'])
        .next()
        .unwrap_or_default();
    hex_id_from_path(candidate).ok_or_else(|| JobsmithError::InvalidVacancyId(input.to_string()))
}

/// Validate and normalize a `{hex}` id (with or without a path prefix).
fn hex_id_from_path(path: &str) -> Option<String> {
    let id = path.rsplit('/').next().unwrap_or_default().trim();
    let looks_hex = (16..=32).contains(&id.len()) && id.chars().all(|c| c.is_ascii_hexdigit());
    looks_hex.then(|| id.to_string())
}

fn selector(css: &str) -> Result<Selector> {
    Selector::parse(css).map_err(|e| JobsmithError::ResponseParse(format!("geekjob selector: {e}")))
}

fn collect_text(el: &scraper::ElementRef<'_>) -> String {
    el.text().collect::<String>().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH: &str = include_str!("../../tests/fixtures/geekjob_search.html");

    #[test]
    fn parses_search_listing() {
        let items = parse_search_html(SEARCH).unwrap();
        assert_eq!(items.len(), 14, "fixture has 14 unique vacancies");
        let first = &items[0];
        assert_eq!(first.id.len(), 24, "hex id: {}", first.id);
        assert!(!first.name.is_empty());
        assert!(
            first
                .alternate_url
                .as_deref()
                .unwrap_or("")
                .starts_with("https://geekjob.ru/vacancy/"),
            "{:?}",
            first.alternate_url
        );
    }

    #[test]
    fn listing_skips_ad_cards_without_vacancy_links() {
        let items = parse_search_html(SEARCH).unwrap();
        assert!(
            items
                .iter()
                .all(|v| v.id.chars().all(|c| c.is_ascii_hexdigit())),
            "every id must be hex"
        );
    }

    #[test]
    fn listing_carries_company_names() {
        let items = parse_search_html(SEARCH).unwrap();
        assert!(
            items.iter().any(|v| v.employer_name() != "Unknown"),
            "at least some cards carry a company name"
        );
    }

    #[test]
    fn extracts_hex_id_from_url_and_bare() {
        assert_eq!(
            extract_hex_id("https://geekjob.ru/vacancy/68396cc09c191336e60ee914").unwrap(),
            "68396cc09c191336e60ee914"
        );
        assert_eq!(
            extract_hex_id("68396cc09c191336e60ee914").unwrap(),
            "68396cc09c191336e60ee914"
        );
        assert!(extract_hex_id("https://geekjob.ru/company/xyz").is_err());
        assert!(extract_hex_id("not-hex").is_err());
        assert!(extract_hex_id("").is_err());
    }
}
