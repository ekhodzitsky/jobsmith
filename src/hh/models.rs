//! HeadHunter API models.
//!
//! All models use `Option<T>` with `skip_serializing_if` for forward compatibility
//! with HH API changes, per AGENTS.md.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Paginated response from the HH vacancies search endpoint.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct VacanciesResponse {
    pub items: Vec<Vacancy>,
    pub found: i32,
    pub pages: i32,
    pub page: i32,
    pub per_page: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alternate_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clusters: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixes: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggests: Option<serde_json::Value>,
}

/// A single vacancy from the HH API.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Vacancy {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub salary: Option<Salary>,
    pub employer: Employer,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<Area>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub vacancy_type: Option<VacancyType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience: Option<NamedEntity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<NamedEntity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub employment: Option<NamedEntity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_skills: Option<Vec<NamedEntity>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alternate_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apply_alternate_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<Address>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snippet: Option<Snippet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_days: Option<Vec<NamedEntity>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_time_intervals: Option<Vec<NamedEntity>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_time_modes: Option<Vec<NamedEntity>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accept_temporary: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub professional_roles: Option<Vec<NamedEntity>>,
    #[serde(default)]
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// Salary information for a vacancy.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Salary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gross: Option<bool>,
}

/// Employer information.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Employer {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alternate_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo_urls: Option<LogoUrls>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vacancies_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trusted: Option<bool>,
}

/// Logo URLs for an employer.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct LogoUrls {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "90")]
    pub size_90: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "240")]
    pub size_240: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original: Option<String>,
}

/// Geographic area (city/region).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Area {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

/// Vacancy type (e.g., open, closed).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct VacancyType {
    pub id: String,
    pub name: String,
}

/// Generic named entity (experience, schedule, skill, etc.).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct NamedEntity {
    pub id: Option<String>,
    pub name: Option<String>,
}

/// Physical address of a vacancy.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Address {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub street: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub building: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lng: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metro: Option<Metro>,
}

/// Metro station information.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Metro {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub station_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub station_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lng: Option<f64>,
}

/// Short text snippet of the vacancy description.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Snippet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirement: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub responsibility: Option<String>,
}

/// Detailed vacancy response (from `/vacancies/{id}`).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct VacancyDetail {
    #[serde(flatten)]
    pub base: Vacancy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contacts: Option<Contacts>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub department: Option<NamedEntity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branded_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_letter_required: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relocation: Option<Relocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Contact information for a vacancy.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Contacts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phones: Option<Vec<Phone>>,
}

/// Phone number.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Phone {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// Relocation information.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Relocation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
}

/// Salary statistics response from HH API.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SalaryStatisticsResponse {
    pub items: Vec<SalaryStatisticsItem>,
}

/// Single salary statistic item.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SalaryStatisticsItem {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub salary: Option<SalaryStatValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percentiles: Option<Percentiles>,
}

/// Salary value with min, max, and average.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct SalaryStatValue {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avg: Option<i32>,
}

/// Salary percentiles.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Percentiles {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "10")]
    pub p10: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "25")]
    pub p25: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "50")]
    pub p50: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "75")]
    pub p75: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "90")]
    pub p90: Option<i32>,
}

/// Search query parameters for HH vacancies.
#[derive(Debug, Clone, Default)]
pub struct VacancySearchQuery {
    pub text: Option<String>,
    pub area: Option<i32>,
    pub experience: Option<String>,
    pub employment: Option<String>,
    pub schedule: Option<String>,
    pub salary: Option<i32>,
    pub currency: Option<String>,
    pub only_with_salary: bool,
    pub page: i32,
    pub per_page: i32,
    pub order_by: Option<String>,
    pub search_field: Option<String>,
    pub professional_role: Option<String>,
}

impl VacancySearchQuery {
    /// Convert to query parameters for reqwest.
    pub fn to_params(&self) -> Vec<(&str, String)> {
        let mut params = Vec::new();
        if let Some(text) = &self.text {
            params.push(("text", text.clone()));
        }
        if let Some(area) = &self.area {
            params.push(("area", area.to_string()));
        }
        if let Some(experience) = &self.experience {
            params.push(("experience", experience.clone()));
        }
        if let Some(employment) = &self.employment {
            params.push(("employment", employment.clone()));
        }
        if let Some(schedule) = &self.schedule {
            params.push(("schedule", schedule.clone()));
        }
        if let Some(salary) = &self.salary {
            params.push(("salary", salary.to_string()));
        }
        if let Some(currency) = &self.currency {
            params.push(("currency", currency.clone()));
        }
        if self.only_with_salary {
            params.push(("only_with_salary", "true".to_string()));
        }
        params.push(("page", self.page.to_string()));
        params.push(("per_page", self.per_page.to_string()));
        if let Some(order_by) = &self.order_by {
            params.push(("order_by", order_by.clone()));
        }
        if let Some(search_field) = &self.search_field {
            params.push(("search_field", search_field.clone()));
        }
        if let Some(professional_role) = &self.professional_role {
            params.push(("professional_role", professional_role.clone()));
        }
        params
    }
}

/// Strip HTML tags from a string, leaving plain text.
pub fn strip_html(html: &str) -> String {
    use scraper::Html;
    let fragment = Html::parse_fragment(html);

    let text = fragment
        .root_element()
        .text()
        .collect::<Vec<_>>()
        .join(" ");

    text.replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn salary_statistics_response_roundtrip() {
        let original = SalaryStatisticsResponse {
            items: vec![SalaryStatisticsItem {
                category: Some("developer".to_string()),
                salary: Some(SalaryStatValue {
                    min: Some(100_000),
                    max: Some(300_000),
                    avg: Some(200_000),
                }),
                percentiles: Some(Percentiles {
                    p10: Some(120_000),
                    p25: Some(150_000),
                    p50: Some(200_000),
                    p75: Some(250_000),
                    p90: Some(280_000),
                }),
            }],
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: SalaryStatisticsResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn salary_statistics_item_roundtrip() {
        let original = SalaryStatisticsItem {
            category: Some("backend".to_string()),
            salary: Some(SalaryStatValue {
                min: Some(50_000),
                max: Some(500_000),
                avg: Some(250_000),
            }),
            percentiles: None,
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: SalaryStatisticsItem = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn salary_stat_value_roundtrip() {
        let original = SalaryStatValue {
            min: Some(100),
            max: Some(1_000),
            avg: Some(500),
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: SalaryStatValue = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn percentiles_roundtrip() {
        let original = Percentiles {
            p10: Some(100),
            p25: Some(200),
            p50: Some(300),
            p75: Some(400),
            p90: Some(500),
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Percentiles = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn vacancy_serde_roundtrip() {
        let vacancy = Vacancy {
            id: "123".to_string(),
            name: "Dev".to_string(),
            description: Some("desc".to_string()),
            salary: Some(Salary {
                from: Some(100),
                to: Some(200),
                currency: Some("RUR".to_string()),
                gross: Some(true),
            }),
            employer: Employer {
                id: "1".to_string(),
                name: "Corp".to_string(),
                url: None,
                alternate_url: None,
                logo_urls: None,
                vacancies_url: None,
                trusted: Some(true),
            },
            area: Some(Area {
                id: "1".to_string(),
                name: "Moscow".to_string(),
                url: None,
                parent_id: None,
            }),
            vacancy_type: Some(VacancyType {
                id: "open".to_string(),
                name: "Open".to_string(),
            }),
            experience: Some(NamedEntity {
                id: Some("1".to_string()),
                name: Some("3-6".to_string()),
            }),
            schedule: None,
            employment: None,
            key_skills: Some(vec![NamedEntity {
                id: Some("1".to_string()),
                name: Some("Rust".to_string()),
            }]),
            published_at: Some(chrono::Utc::now()),
            created_at: Some(chrono::Utc::now()),
            alternate_url: None,
            apply_alternate_url: None,
            address: None,
            snippet: None,
            working_days: None,
            working_time_intervals: None,
            working_time_modes: None,
            accept_temporary: None,
            professional_roles: None,
            extra: serde_json::Value::Object(Default::default()),
        };

        let json = serde_json::to_string(&vacancy).unwrap();
        let decoded: Vacancy = serde_json::from_str(&json).unwrap();
        assert_eq!(vacancy, decoded);
    }

    #[test]
    fn vacancy_detail_serde_roundtrip() {
        let detail = VacancyDetail {
            base: Vacancy {
                id: "456".to_string(),
                name: "Senior Dev".to_string(),
                description: None,
                salary: None,
                employer: Employer {
                    id: "2".to_string(),
                    name: "Other".to_string(),
                    url: None,
                    alternate_url: None,
                    logo_urls: None,
                    vacancies_url: None,
                    trusted: None,
                },
                area: None,
                vacancy_type: None,
                experience: None,
                schedule: None,
                employment: None,
                key_skills: None,
                published_at: None,
                created_at: None,
                alternate_url: None,
                apply_alternate_url: None,
                address: None,
                snippet: None,
                working_days: None,
                working_time_intervals: None,
                working_time_modes: None,
                accept_temporary: None,
                professional_roles: None,
                extra: serde_json::Value::Object(Default::default()),
            },
            contacts: Some(Contacts {
                name: Some("HR".to_string()),
                email: Some("hr@example.com".to_string()),
                phones: Some(vec![Phone {
                    country: Some("7".to_string()),
                    city: Some("495".to_string()),
                    number: Some("1234567".to_string()),
                    comment: None,
                }]),
            }),
            department: Some(NamedEntity {
                id: Some("dep".to_string()),
                name: Some("Engineering".to_string()),
            }),
            branded_description: Some("brand".to_string()),
            hidden: Some(false),
            response_letter_required: Some(true),
            relocation: Some(Relocation {
                type_id: Some("1".to_string()),
                type_name: Some("Possible".to_string()),
            }),
            request_id: Some("req".to_string()),
        };

        let json = serde_json::to_string(&detail).unwrap();
        let decoded: VacancyDetail = serde_json::from_str(&json).unwrap();
        assert_eq!(detail, decoded);
    }

    #[test]
    fn salary_serde_roundtrip() {
        let salary = Salary {
            from: Some(100_000),
            to: Some(200_000),
            currency: Some("RUR".to_string()),
            gross: Some(false),
        };
        let json = serde_json::to_string(&salary).unwrap();
        let decoded: Salary = serde_json::from_str(&json).unwrap();
        assert_eq!(salary, decoded);
    }

    #[test]
    fn employer_serde_roundtrip() {
        let employer = Employer {
            id: "1".to_string(),
            name: "Yandex".to_string(),
            url: Some("https://yandex.ru".to_string()),
            alternate_url: None,
            logo_urls: Some(LogoUrls {
                size_90: Some("https://example.com/90.png".to_string()),
                size_240: None,
                original: Some("https://example.com/orig.png".to_string()),
            }),
            vacancies_url: Some("https://api.hh.ru/vacancies?employer_id=1".to_string()),
            trusted: Some(true),
        };
        let json = serde_json::to_string(&employer).unwrap();
        let decoded: Employer = serde_json::from_str(&json).unwrap();
        assert_eq!(employer, decoded);
    }

    #[test]
    fn area_serde_roundtrip() {
        let area = Area {
            id: "1".to_string(),
            name: "Moscow".to_string(),
            url: Some("https://api.hh.ru/areas/1".to_string()),
            parent_id: Some("0".to_string()),
        };
        let json = serde_json::to_string(&area).unwrap();
        let decoded: Area = serde_json::from_str(&json).unwrap();
        assert_eq!(area, decoded);
    }

    #[test]
    fn vacancies_response_serde_roundtrip() {
        let response = VacanciesResponse {
            items: vec![],
            found: 0,
            pages: 1,
            page: 0,
            per_page: 20,
            alternate_url: None,
            arguments: None,
            clusters: None,
            fixes: None,
            suggests: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        let decoded: VacanciesResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response, decoded);
    }

    #[test]
    fn named_entity_serde_roundtrip() {
        let entity = NamedEntity {
            id: Some("1".to_string()),
            name: Some("Full time".to_string()),
        };
        let json = serde_json::to_string(&entity).unwrap();
        let decoded: NamedEntity = serde_json::from_str(&json).unwrap();
        assert_eq!(entity, decoded);
    }

    #[test]
    fn address_serde_roundtrip() {
        let address = Address {
            city: Some("Moscow".to_string()),
            street: Some("Tverskaya".to_string()),
            building: Some("1".to_string()),
            lat: Some(55.75),
            lng: Some(37.62),
            raw: Some("Moscow, Tverskaya 1".to_string()),
            metro: Some(Metro {
                station_name: Some("Okhotny Ryad".to_string()),
                line_name: Some("Sokolnicheskaya".to_string()),
                station_id: Some("1".to_string()),
                line_id: Some("1".to_string()),
                lat: Some(55.75),
                lng: Some(37.62),
            }),
        };
        let json = serde_json::to_string(&address).unwrap();
        let decoded: Address = serde_json::from_str(&json).unwrap();
        assert_eq!(address, decoded);
    }

    #[test]
    fn snippet_serde_roundtrip() {
        let snippet = Snippet {
            requirement: Some("3+ years".to_string()),
            responsibility: Some("Develop features".to_string()),
        };
        let json = serde_json::to_string(&snippet).unwrap();
        let decoded: Snippet = serde_json::from_str(&json).unwrap();
        assert_eq!(snippet, decoded);
    }

    #[test]
    fn contacts_serde_roundtrip() {
        let contacts = Contacts {
            name: Some("HR".to_string()),
            email: Some("hr@example.com".to_string()),
            phones: Some(vec![Phone {
                country: Some("7".to_string()),
                city: Some("495".to_string()),
                number: Some("1234567".to_string()),
                comment: None,
            }]),
        };
        let json = serde_json::to_string(&contacts).unwrap();
        let decoded: Contacts = serde_json::from_str(&json).unwrap();
        assert_eq!(contacts, decoded);
    }

    #[test]
    fn relocation_serde_roundtrip() {
        let relocation = Relocation {
            type_id: Some("1".to_string()),
            type_name: Some("Possible".to_string()),
        };
        let json = serde_json::to_string(&relocation).unwrap();
        let decoded: Relocation = serde_json::from_str(&json).unwrap();
        assert_eq!(relocation, decoded);
    }

    #[test]
    fn strip_html_removes_tags() {
        let html = "<p>Hello <b>world</b></p>";
        let text = strip_html(html);
        assert_eq!(text, "Hello world");
    }

    #[test]
    fn strip_html_handles_plain_text() {
        let html = "plain text";
        let text = strip_html(html);
        assert_eq!(text, "plain text");
    }
}
