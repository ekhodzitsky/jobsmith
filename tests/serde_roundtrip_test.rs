use chrono::Utc;
use jobsmith::hh::models::*;

#[test]
fn vacancies_response_roundtrip() {
    let original = VacanciesResponse {
        items: vec![Vacancy {
            id: "1".to_string(),
            name: "Test".to_string(),
            description: Some("desc".to_string()),
            salary: Some(Salary {
                from: Some(100),
                to: Some(200),
                currency: Some("RUR".to_string()),
                gross: Some(true),
            }),
            employer: Employer {
                id: "1".to_string(),
                name: "Employer".to_string(),
                url: Some("url".to_string()),
                alternate_url: Some("alt".to_string()),
                logo_urls: Some(LogoUrls {
                    size_90: Some("90".to_string()),
                    size_240: Some("240".to_string()),
                    original: Some("orig".to_string()),
                }),
                vacancies_url: Some("vac".to_string()),
                trusted: Some(true),
            },
            area: Some(Area {
                id: "1".to_string(),
                name: "Moscow".to_string(),
                url: Some("url".to_string()),
                parent_id: Some("0".to_string()),
            }),
            vacancy_type: Some(VacancyType {
                id: "1".to_string(),
                name: "Open".to_string(),
            }),
            experience: Some(NamedEntity {
                id: Some("1".to_string()),
                name: Some("3-6".to_string()),
            }),
            schedule: Some(NamedEntity {
                id: Some("1".to_string()),
                name: Some("full".to_string()),
            }),
            employment: Some(NamedEntity {
                id: Some("1".to_string()),
                name: Some("full".to_string()),
            }),
            key_skills: Some(vec![NamedEntity {
                id: Some("1".to_string()),
                name: Some("Rust".to_string()),
            }]),
            published_at: Some(Utc::now()),
            created_at: Some(Utc::now()),
            alternate_url: Some("alt".to_string()),
            apply_alternate_url: Some("apply".to_string()),
            address: Some(Address {
                city: Some("Moscow".to_string()),
                street: Some("Main".to_string()),
                building: Some("1".to_string()),
                lat: Some(55.0),
                lng: Some(37.0),
                raw: Some("raw".to_string()),
                metro: Some(Metro {
                    station_name: Some("Station".to_string()),
                    line_name: Some("Line".to_string()),
                    station_id: Some("1".to_string()),
                    line_id: Some("1".to_string()),
                    lat: Some(55.0),
                    lng: Some(37.0),
                }),
            }),
            snippet: Some(Snippet {
                requirement: Some("req".to_string()),
                responsibility: Some("resp".to_string()),
            }),
            working_days: Some(vec![NamedEntity {
                id: Some("1".to_string()),
                name: Some("Mon".to_string()),
            }]),
            working_time_intervals: Some(vec![NamedEntity {
                id: Some("1".to_string()),
                name: Some("day".to_string()),
            }]),
            working_time_modes: Some(vec![NamedEntity {
                id: Some("1".to_string()),
                name: Some("office".to_string()),
            }]),
            accept_temporary: Some(false),
            professional_roles: Some(vec![NamedEntity {
                id: Some("96".to_string()),
                name: Some("Developer".to_string()),
            }]),
            extra: serde_json::Value::Null,
        }],
        found: 1,
        pages: 1,
        page: 0,
        per_page: 20,
        alternate_url: Some("alt".to_string()),
        arguments: None,
        clusters: None,
        fixes: None,
        suggests: None,
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: VacanciesResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn vacancy_roundtrip() {
    let original = Vacancy {
        id: "1".to_string(),
        name: "Test".to_string(),
        description: None,
        salary: None,
        employer: Employer {
            id: "1".to_string(),
            name: "Test".to_string(),
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
        extra: serde_json::Value::Null,
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Vacancy = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn salary_roundtrip() {
    let original = Salary {
        from: Some(100),
        to: Some(200),
        currency: Some("RUR".to_string()),
        gross: Some(true),
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Salary = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn employer_roundtrip() {
    let original = Employer {
        id: "1".to_string(),
        name: "Test".to_string(),
        url: None,
        alternate_url: None,
        logo_urls: None,
        vacancies_url: None,
        trusted: None,
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Employer = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn logo_urls_roundtrip() {
    let original = LogoUrls {
        size_90: Some("90".to_string()),
        size_240: Some("240".to_string()),
        original: Some("orig".to_string()),
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: LogoUrls = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn area_roundtrip() {
    let original = Area {
        id: "1".to_string(),
        name: "Moscow".to_string(),
        url: None,
        parent_id: None,
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Area = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn vacancy_type_roundtrip() {
    let original = VacancyType {
        id: "1".to_string(),
        name: "Open".to_string(),
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: VacancyType = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn named_entity_roundtrip() {
    let original = NamedEntity {
        id: Some("1".to_string()),
        name: Some("Test".to_string()),
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: NamedEntity = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn address_roundtrip() {
    let original = Address {
        city: Some("Moscow".to_string()),
        street: Some("Main".to_string()),
        building: Some("1".to_string()),
        lat: Some(55.0),
        lng: Some(37.0),
        raw: Some("raw".to_string()),
        metro: None,
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Address = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn metro_roundtrip() {
    let original = Metro {
        station_name: Some("Station".to_string()),
        line_name: Some("Line".to_string()),
        station_id: Some("1".to_string()),
        line_id: Some("1".to_string()),
        lat: Some(55.0),
        lng: Some(37.0),
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Metro = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn snippet_roundtrip() {
    let original = Snippet {
        requirement: Some("req".to_string()),
        responsibility: Some("resp".to_string()),
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Snippet = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn vacancy_detail_roundtrip() {
    let original = VacancyDetail {
        base: Vacancy {
            id: "1".to_string(),
            name: "Test".to_string(),
            description: None,
            salary: None,
            employer: Employer {
                id: "1".to_string(),
                name: "Test".to_string(),
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
            extra: serde_json::Value::Null,
        },
        contacts: Some(Contacts {
            name: Some("Contact".to_string()),
            email: Some("email@example.com".to_string()),
            phones: Some(vec![Phone {
                country: Some("7".to_string()),
                city: Some("495".to_string()),
                number: Some("1234567".to_string()),
                comment: None,
            }]),
        }),
        department: Some(NamedEntity {
            id: Some("1".to_string()),
            name: Some("Dept".to_string()),
        }),
        branded_description: Some("brand".to_string()),
        hidden: Some(false),
        response_letter_required: Some(true),
        relocation: Some(Relocation {
            type_id: Some("1".to_string()),
            type_name: Some("Relo".to_string()),
        }),
        request_id: Some("req-id".to_string()),
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: VacancyDetail = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn contacts_roundtrip() {
    let original = Contacts {
        name: Some("Contact".to_string()),
        email: Some("email@example.com".to_string()),
        phones: Some(vec![Phone {
            country: Some("7".to_string()),
            city: Some("495".to_string()),
            number: Some("1234567".to_string()),
            comment: None,
        }]),
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Contacts = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn phone_roundtrip() {
    let original = Phone {
        country: Some("7".to_string()),
        city: Some("495".to_string()),
        number: Some("1234567".to_string()),
        comment: None,
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Phone = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

#[test]
fn relocation_roundtrip() {
    let original = Relocation {
        type_id: Some("1".to_string()),
        type_name: Some("Relo".to_string()),
    };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Relocation = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}

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
