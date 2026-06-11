//! Models and parsing for the Trudvsem open-data API.
//!
//! Response shape (see <https://trudvsem.ru/opendata/api>):
//! `{ "status": "200", "meta": {...}, "results": { "vacancies": [ {"vacancy": {...}} ] } }`.

use serde::{Deserialize, Serialize};

use crate::error::{JobsmithError, Result};
use crate::hh::models::{Area, Employer, NamedEntity, Salary, Vacancy, VacancyDetail};

/// Top-level API response.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct TrudvsemResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<TrudvsemMeta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<TrudvsemResults>,
}

/// Pagination metadata.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct TrudvsemMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
}

/// `results` container.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct TrudvsemResults {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vacancies: Vec<TrudvsemVacancyWrapper>,
}

/// Each array element wraps the vacancy under a `vacancy` key.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct TrudvsemVacancyWrapper {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vacancy: Option<TrudvsemVacancy>,
}

/// A single Trudvsem vacancy.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct TrudvsemVacancy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(rename = "job-name", default, skip_serializing_if = "Option::is_none")]
    pub job_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duty: Option<String>,
    #[serde(
        rename = "creation-date",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub creation_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub salary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub salary_min: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub salary_max: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<TrudvsemRegion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company: Option<TrudvsemCompany>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirement: Option<TrudvsemRequirement>,
    /// Free-form list; kept as raw JSON for forward compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub employment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vac_url: Option<String>,
}

/// Region info.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct TrudvsemRegion {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Employer info.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct TrudvsemCompany {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companycode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inn: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ogrn: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(rename = "hr-agency", default, skip_serializing_if = "Option::is_none")]
    pub hr_agency: Option<bool>,
}

/// Candidate requirements.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct TrudvsemRequirement {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub education: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience: Option<i64>,
}

/// Parse a search (or detail) API response into vacancies.
pub fn parse_vacancies(json: &str) -> Result<Vec<Vacancy>> {
    let response: TrudvsemResponse = serde_json::from_str(json)
        .map_err(|e| JobsmithError::ResponseParse(format!("trudvsem: {e}")))?;
    Ok(response
        .results
        .map(|r| r.vacancies)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|w| w.vacancy)
        .filter_map(TrudvsemVacancy::into_vacancy)
        .collect())
}

/// Parse a detail API response into a single [`VacancyDetail`].
pub fn parse_vacancy_detail(json: &str) -> Result<VacancyDetail> {
    let base = parse_vacancies(json)?.into_iter().next().ok_or_else(|| {
        JobsmithError::ResponseParse("trudvsem: detail response has no vacancy".to_string())
    })?;
    Ok(VacancyDetail {
        base,
        ..Default::default()
    })
}

/// Extract the `{companyCode}/{vacancyId}` pair from a Trudvsem card
/// URL (`https://trudvsem.ru/vacancy/card/{cc}/{vid}`) or an already
/// combined `cc/vid` string.
pub fn extract_card_ids(input: &str) -> Result<(String, String)> {
    let path = input
        .split_once("/vacancy/card/")
        .map_or(input, |(_, tail)| tail);
    let mut segments = path.trim_matches('/').split('/').filter(|s| !s.is_empty());
    match (segments.next(), segments.next(), segments.next()) {
        (Some(cc), Some(vid), None) => {
            // strip a possible ?query/#fragment from the last segment
            let vid = vid.split(['?', '#']).next().unwrap_or_default();
            if cc.is_empty() || vid.is_empty() {
                return Err(JobsmithError::InvalidVacancyId(input.to_string()));
            }
            Ok((cc.to_string(), vid.to_string()))
        }
        _ => Err(JobsmithError::InvalidVacancyId(input.to_string())),
    }
}

impl TrudvsemVacancy {
    /// Map into the shared [`Vacancy`] model; `None` without an id or title.
    fn into_vacancy(self) -> Option<Vacancy> {
        let id = self.id.as_deref()?.trim().to_string();
        let name = self.job_name.as_deref()?.trim().to_string();
        if id.is_empty() || name.is_empty() {
            return None;
        }
        let salary = self.salary_range();
        let description = self.description();
        let employer = self.company.as_ref().and_then(|c| {
            c.name.as_deref().map(|n| Employer {
                id: c.companycode.clone(),
                name: Some(n.to_string()),
                url: c.url.clone(),
                ..Default::default()
            })
        });
        let area = self.region.and_then(|r| {
            r.name.map(|name| Area {
                id: r.region_code,
                name: Some(name),
                url: None,
                parent_id: None,
            })
        });
        let schedule = self.schedule.map(|name| NamedEntity {
            id: None,
            name: Some(name),
        });
        let employment = self.employment.map(|name| NamedEntity {
            id: None,
            name: Some(name),
        });

        Some(Vacancy {
            id,
            name,
            description,
            salary,
            employer,
            area,
            schedule,
            employment,
            alternate_url: self.vac_url,
            ..Default::default()
        })
    }

    /// Build [`Salary`] from the min/max bounds; `0` means "unspecified".
    fn salary_range(&self) -> Option<Salary> {
        let bound = |v: Option<i64>| v.filter(|n| *n > 0).and_then(|n| i32::try_from(n).ok());
        let from = bound(self.salary_min);
        let to = bound(self.salary_max);
        if from.is_none() && to.is_none() {
            return None;
        }
        let currency = self
            .currency
            .as_deref()
            .map(|c| c.trim_matches(['«', '»', ' ']).to_string())
            .filter(|c| !c.is_empty());
        Some(Salary {
            from,
            to,
            currency,
            gross: None,
        })
    }

    /// Assemble the AI-facing description from duty + requirements + skills.
    fn description(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();
        if let Some(duty) = self
            .duty
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            parts.push(duty.to_string());
        }
        if let Some(req) = &self.requirement {
            let mut req_bits = Vec::new();
            if let Some(edu) = req.education.as_deref().filter(|s| !s.is_empty()) {
                req_bits.push(format!("образование — {edu}"));
            }
            if let Some(exp) = req.experience.filter(|e| *e > 0) {
                req_bits.push(format!("опыт от {exp} лет"));
            }
            if !req_bits.is_empty() {
                parts.push(format!("Требования: {}.", req_bits.join(", ")));
            }
        }
        if let Some(skills) = self.skills.as_ref().and_then(skill_list) {
            parts.push(format!("Навыки: {skills}."));
        }
        (!parts.is_empty()).then(|| parts.join("\n\n"))
    }
}

/// Pull a comma-joined skill list out of the free-form `skills` value.
fn skill_list(value: &serde_json::Value) -> Option<String> {
    let items: Vec<&str> = value
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    (!items.is_empty()).then(|| items.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH: &str = include_str!("../../tests/fixtures/trudvsem_search.json");
    const DETAIL: &str = include_str!("../../tests/fixtures/trudvsem_vacancy.json");

    #[test]
    fn parses_all_search_items() {
        let items = parse_vacancies(SEARCH).unwrap();
        assert_eq!(items.len(), 4, "fixture has 4 vacancies");
    }

    #[test]
    fn maps_first_item_fields() {
        let items = parse_vacancies(SEARCH).unwrap();
        let first = &items[0];
        assert_eq!(first.id, "58c3f1e4-5b4a-11f1-b964-dd82fad7662e");
        assert!(first.name.contains("Rust TechLead"), "name: {}", first.name);
        assert_eq!(first.employer_name(), "МТС");
        assert_eq!(
            first.area.as_ref().and_then(|a| a.name.as_deref()),
            Some("Город Москва")
        );
        assert!(first
            .alternate_url
            .as_deref()
            .unwrap_or("")
            .contains("/vacancy/card/7226c750-02f1-11eb-8600-bfd13399602c/"));
        assert!(
            first.description.as_deref().unwrap_or("").len() > 500,
            "duty must flow into description"
        );
    }

    #[test]
    fn salary_zero_bounds_become_none() {
        let items = parse_vacancies(SEARCH).unwrap();
        // first fixture item: salary_min=30000, salary_max=0
        let salary = items[0].salary.as_ref().expect("salary present");
        assert_eq!(salary.from, Some(30_000));
        assert_eq!(salary.to, None, "salary_max=0 means unspecified");
        // currency «руб.» is cleaned of the guillemets
        assert_eq!(salary.currency.as_deref(), Some("руб."));
    }

    #[test]
    fn detail_fixture_maps_to_vacancy_detail() {
        let detail = parse_vacancy_detail(DETAIL).unwrap();
        assert!(detail.base.name.contains("Rust TechLead"));
        assert_eq!(detail.employer_name(), "МТС");
        assert!(detail.base.description.is_some());
    }

    #[test]
    fn extract_card_ids_from_url_and_pair() {
        let url = "https://trudvsem.ru/vacancy/card/7226c750-02f1-11eb-8600-bfd13399602c/58c3f1e4-5b4a-11f1-b964-dd82fad7662e";
        let (cc, vid) = extract_card_ids(url).unwrap();
        assert_eq!(cc, "7226c750-02f1-11eb-8600-bfd13399602c");
        assert_eq!(vid, "58c3f1e4-5b4a-11f1-b964-dd82fad7662e");

        let (cc2, vid2) = extract_card_ids("aaa/bbb").unwrap();
        assert_eq!((cc2.as_str(), vid2.as_str()), ("aaa", "bbb"));

        assert!(extract_card_ids("just-one-segment").is_err());
        assert!(extract_card_ids("").is_err());
    }

    #[test]
    fn trudvsem_response_roundtrip() {
        let original = TrudvsemResponse {
            status: Some("200".to_string()),
            meta: Some(TrudvsemMeta {
                total: Some(4),
                limit: Some(5),
            }),
            results: Some(TrudvsemResults {
                vacancies: vec![TrudvsemVacancyWrapper {
                    vacancy: Some(TrudvsemVacancy {
                        id: Some("vid".to_string()),
                        job_name: Some("Rust Dev".to_string()),
                        salary_min: Some(100),
                        region: Some(TrudvsemRegion {
                            region_code: Some("77".to_string()),
                            name: Some("Москва".to_string()),
                        }),
                        company: Some(TrudvsemCompany {
                            name: Some("МТС".to_string()),
                            companycode: Some("cc".to_string()),
                            ..Default::default()
                        }),
                        requirement: Some(TrudvsemRequirement {
                            education: Some("Высшее".to_string()),
                            experience: Some(3),
                        }),
                        ..Default::default()
                    }),
                }],
            }),
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: TrudvsemResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn vacancy_model_renamed_fields_roundtrip() {
        let original = TrudvsemVacancy {
            job_name: Some("Dev".to_string()),
            creation_date: Some("2026-06-01".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_string(&original).unwrap();
        assert!(json.contains("\"job-name\""), "{json}");
        assert!(json.contains("\"creation-date\""), "{json}");
        let decoded: TrudvsemVacancy = serde_json::from_str(&json).unwrap();
        assert_eq!(original, decoded);
    }
}
