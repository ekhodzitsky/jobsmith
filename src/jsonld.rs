//! Shared extractor for schema.org `JobPosting` JSON-LD blocks.
//!
//! Several job boards (Habr Career, GeekJob) publish the same
//! structured data they expose to search engines; reading it is far
//! more change-resistant than scraping the rendered DOM.

use scraper::{Html, Selector};
use serde_json::Value;

use crate::hh::models::{Area, Employer, Salary};

/// Find the `JobPosting` object among the page's JSON-LD blocks.
///
/// Handles both a bare object and a `@graph` array wrapper.
pub(crate) fn find_job_posting(html: &str) -> Option<Value> {
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

/// A non-empty string field of the posting.
pub(crate) fn str_field(posting: &Value, key: &str) -> Option<String> {
    posting
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

/// `hiringOrganization` → [`Employer`].
pub(crate) fn employer_from(posting: &Value) -> Option<Employer> {
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

/// `baseSalary` (schema.org `MonetaryAmount`) → [`Salary`].
///
/// Bounds may live in `value.{minValue,maxValue}` (QuantitativeValue),
/// be a single `value` number, or sit on the amount itself.
pub(crate) fn salary_from(posting: &Value) -> Option<Salary> {
    let amount = posting.get("baseSalary")?;
    let value = amount.get("value");
    let bound = |key: &str| -> Option<i32> {
        let node = value.and_then(|v| v.get(key)).or_else(|| amount.get(key))?;
        let n = node.as_i64().or_else(|| node.as_f64().map(|f| f as i64))?;
        i32::try_from(n).ok().filter(|n| *n > 0)
    };
    let single = || -> Option<i32> {
        let n = value?.as_i64()?;
        i32::try_from(n).ok().filter(|n| *n > 0)
    };
    let from = bound("minValue").or_else(single);
    let to = bound("maxValue");
    if from.is_none() && to.is_none() {
        return None;
    }
    let currency = amount
        .get("currency")
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|c| !c.is_empty());
    Some(Salary {
        from,
        to,
        currency,
        gross: None,
    })
}

/// `jobLocation` → [`Area`].
pub(crate) fn area_from(posting: &Value) -> Option<Area> {
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
