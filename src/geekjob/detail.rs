//! Parser for a GeekJob vacancy page (schema.org JSON-LD).

use crate::error::{JobsmithError, Result};
use crate::hh::models::{Vacancy, VacancyDetail};
use crate::jsonld;

const GEEKJOB_VACANCY_BASE: &str = "https://geekjob.ru/vacancy";

/// Parse a GeekJob vacancy page into a [`VacancyDetail`].
pub fn parse_vacancy_html(html: &str, id: &str) -> Result<VacancyDetail> {
    let posting = jsonld::find_job_posting(html).ok_or_else(|| {
        JobsmithError::ResponseParse("geekjob: no JobPosting JSON-LD".to_string())
    })?;

    let name = jsonld::str_field(&posting, "title").ok_or_else(|| {
        JobsmithError::ResponseParse("geekjob: JobPosting has no title".to_string())
    })?;

    Ok(VacancyDetail {
        base: Vacancy {
            id: id.to_string(),
            name,
            description: jsonld::str_field(&posting, "description"),
            employer: jsonld::employer_from(&posting),
            area: jsonld::area_from(&posting),
            salary: jsonld::salary_from(&posting),
            alternate_url: Some(format!("{GEEKJOB_VACANCY_BASE}/{id}")),
            ..Default::default()
        },
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = include_str!("../../tests/fixtures/geekjob_vacancy.html");
    const PAGE_WITH_SALARY: &str = include_str!("../../tests/fixtures/geekjob_vacancy_salary.html");

    #[test]
    fn extracts_core_fields() {
        let detail = parse_vacancy_html(PAGE, "68396cc09c191336e60ee914").unwrap();
        assert!(
            detail.base.name.contains("Ruby on Rails"),
            "{}",
            detail.base.name
        );
        assert_eq!(detail.employer_name(), "Агентство NEWHR");
        assert_eq!(
            detail.base.alternate_url.as_deref(),
            Some("https://geekjob.ru/vacancy/68396cc09c191336e60ee914")
        );
        assert!(detail.base.description.is_some());
    }

    #[test]
    fn maps_base_salary_to_salary() {
        let detail = parse_vacancy_html(PAGE_WITH_SALARY, "691aef77ce60ad56c3046275").unwrap();
        let salary = detail.base.salary.as_ref().expect("salary present");
        assert_eq!(salary.from, Some(4_000));
        assert_eq!(salary.to, Some(6_500));
        assert_eq!(salary.currency.as_deref(), Some("EUR"));
    }

    #[test]
    fn missing_job_posting_errors() {
        let err = parse_vacancy_html("<html></html>", "1").unwrap_err();
        assert!(matches!(err, JobsmithError::ResponseParse(_)), "{err:?}");
    }
}
