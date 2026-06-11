//! Parser for the Habr Career RSS search feed.

use std::sync::LazyLock;

use quick_xml::events::Event;
use quick_xml::Reader;
use regex::Regex;

use crate::error::{JobsmithError, Result};
use crate::hh::models::{Area, Employer, Snippet, Vacancy};

/// The role title is quoted in `«...»`; the city trails in `(...)`.
static ROLE_RE: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"«(.+?)»").ok());
static CITY_RE: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"\(([^)]+)\)\s*$").ok());

/// Accumulates the fields of a single `<item>` as they stream in.
#[derive(Default)]
struct ItemAcc {
    title: String,
    description: String,
    author: String,
    link: String,
    guid: String,
}

impl ItemAcc {
    fn field_mut(&mut self, tag: &[u8]) -> Option<&mut String> {
        match tag {
            b"title" => Some(&mut self.title),
            b"description" => Some(&mut self.description),
            b"author" => Some(&mut self.author),
            b"link" => Some(&mut self.link),
            b"guid" => Some(&mut self.guid),
            _ => None,
        }
    }

    fn into_vacancy(self) -> Option<Vacancy> {
        let id = first_non_empty(&[self.guid.trim(), last_path_segment(&self.link)])?;
        let name = extract_role(&self.title);
        let employer = non_empty(self.author.trim()).map(|name| Employer {
            id: None,
            name: Some(name),
            ..Default::default()
        });
        let area = extract_city(&self.title).map(|name| Area {
            id: None,
            name: Some(name),
            url: None,
            parent_id: None,
        });
        let snippet = non_empty(self.description.trim()).map(|text| Snippet {
            requirement: Some(text),
            responsibility: None,
        });

        Some(Vacancy {
            id,
            name,
            employer,
            area,
            snippet,
            alternate_url: non_empty(self.link.trim()),
            ..Default::default()
        })
    }
}

/// Parse a Habr Career RSS search feed into vacancy listings.
///
/// Each `<item>` carries a `guid` (vacancy id), `title` (the role,
/// quoted), `author` (the company), `link` and a short `description`.
/// The richer fields (salary, full description) are only available from
/// the detail page, so they stay `None` here.
pub fn parse_search_rss(xml: &str) -> Result<Vec<Vacancy>> {
    let mut reader = Reader::from_str(xml);
    let mut vacancies = Vec::new();
    let mut item: Option<ItemAcc> = None;
    let mut current_tag: Vec<u8> = Vec::new();

    loop {
        match reader
            .read_event()
            .map_err(|e| JobsmithError::ResponseParse(format!("habr rss: {e}")))?
        {
            Event::Start(e) if e.name().as_ref() == b"item" => {
                item = Some(ItemAcc::default());
                current_tag.clear();
            }
            Event::End(e) if e.name().as_ref() == b"item" => {
                if let Some(acc) = item.take() {
                    if let Some(vacancy) = acc.into_vacancy() {
                        vacancies.push(vacancy);
                    }
                }
                current_tag.clear();
            }
            Event::Start(e) if item.is_some() => {
                current_tag = e.name().as_ref().to_vec();
            }
            Event::End(_) if item.is_some() => {
                current_tag.clear();
            }
            Event::Text(e) if item.is_some() && !current_tag.is_empty() => {
                let decoded = e
                    .decode()
                    .map_err(|err| JobsmithError::ResponseParse(format!("habr rss: {err}")))?;
                let text = quick_xml::escape::unescape(&decoded)
                    .map_err(|err| JobsmithError::ResponseParse(format!("habr rss: {err}")))?;
                if let Some(field) = item.as_mut().and_then(|i| i.field_mut(&current_tag)) {
                    field.push_str(&text);
                }
            }
            Event::CData(e) if item.is_some() && !current_tag.is_empty() => {
                let text = e
                    .decode()
                    .map_err(|err| JobsmithError::ResponseParse(format!("habr rss: {err}")))?;
                if let Some(field) = item.as_mut().and_then(|i| i.field_mut(&current_tag)) {
                    field.push_str(&text);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(vacancies)
}

fn extract_role(title: &str) -> String {
    ROLE_RE
        .as_ref()
        .and_then(|re| re.captures(title))
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| title.trim().to_string())
}

fn extract_city(title: &str) -> Option<String> {
    CITY_RE
        .as_ref()
        .and_then(|re| re.captures(title))
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
}

fn last_path_segment(url: &str) -> &str {
    url.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .trim()
}

fn non_empty(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

fn first_non_empty(candidates: &[&str]) -> Option<String> {
    candidates.iter().find_map(|c| non_empty(c.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEED: &str = include_str!("../../tests/fixtures/habr_search.rss");

    #[test]
    fn parses_all_items() {
        let items = parse_search_rss(FEED).unwrap();
        assert_eq!(items.len(), 9, "fixture has 9 vacancies");
    }

    #[test]
    fn maps_first_item_fields() {
        let items = parse_search_rss(FEED).unwrap();
        let first = &items[0];
        assert_eq!(first.id, "1000166679");
        assert!(first.name.contains("Rust"), "name: {}", first.name);
        assert_eq!(first.employer_name(), "Bell Integrator");
        assert_eq!(
            first.alternate_url.as_deref(),
            Some("https://career.habr.com/vacancies/1000166679")
        );
        let snippet = first.snippet.as_ref().expect("short description");
        assert!(
            snippet
                .requirement
                .as_deref()
                .unwrap_or("")
                .contains("Bell Integrator"),
            "snippet: {:?}",
            snippet.requirement
        );
    }

    #[test]
    fn extracts_city_from_title() {
        let items = parse_search_rss(FEED).unwrap();
        let city = items[0].area.as_ref().and_then(|a| a.name.as_deref());
        assert_eq!(city, Some("Москва"));
    }

    #[test]
    fn empty_feed_yields_no_items() {
        let xml = r#"<?xml version="1.0"?><rss version="2.0"><channel></channel></rss>"#;
        assert!(parse_search_rss(xml).unwrap().is_empty());
    }

    #[test]
    fn role_falls_back_to_full_title_without_quotes() {
        assert_eq!(extract_role("Rust Developer"), "Rust Developer");
        assert_eq!(extract_role("Требуется «Rust» (Москва)"), "Rust");
    }
}
