use jobsmith::hh::models::*;
use jobsmith::hh::HhClient;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn make_minimal_vacancy() -> Vacancy {
    Vacancy {
        id: "123".to_string(),
        name: "Rust Developer".to_string(),
        description: None,
        salary: None,
        employer: Some(Employer {
            id: Some("1".to_string()),
            name: Some("Test Corp".to_string()),
            url: None,
            alternate_url: None,
            logo_urls: None,
            vacancies_url: None,
            trusted: None,
        }),
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
    }
}

#[tokio::test]
async fn test_search_vacancies() {
    let server = MockServer::start().await;
    let client = HhClient::with_base_url(server.uri()).unwrap();

    let mock_response = VacanciesResponse {
        items: vec![make_minimal_vacancy()],
        found: 1,
        pages: 1,
        page: 0,
        per_page: 20,
        alternate_url: None,
        arguments: None,
        clusters: None,
        fixes: None,
        suggests: None,
    };

    Mock::given(method("GET"))
        .and(path("/vacancies"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
        .mount(&server)
        .await;

    let query = VacancySearchQuery {
        text: Some("rust".to_string()),
        ..VacancySearchQuery::default()
    };

    let result = client.search_vacancies(&query).await.unwrap();
    assert_eq!(result.found, 1);
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].name, "Rust Developer");
}

#[tokio::test]
async fn test_get_vacancy() {
    let server = MockServer::start().await;
    let client = HhClient::with_base_url(server.uri()).unwrap();

    let mut vacancy = make_minimal_vacancy();
    vacancy.id = "456".to_string();
    vacancy.name = "Senior Rust Developer".to_string();
    vacancy.salary = Some(Salary {
        from: Some(300_000),
        to: Some(500_000),
        currency: Some("RUR".to_string()),
        gross: Some(true),
    });

    let mock_response = VacancyDetail {
        base: vacancy,
        contacts: None,
        department: None,
        branded_description: None,
        hidden: None,
        response_letter_required: None,
        relocation: None,
        request_id: None,
    };

    Mock::given(method("GET"))
        .and(path("/vacancies/456"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
        .mount(&server)
        .await;

    let result = client.get_vacancy("456").await.unwrap();
    assert_eq!(result.base.name, "Senior Rust Developer");
    assert_eq!(result.base.salary.as_ref().unwrap().from, Some(300_000));
}

#[tokio::test]
async fn test_get_salary_statistics() {
    let server = MockServer::start().await;
    let client = HhClient::with_base_url(server.uri()).unwrap();

    let mock_response = SalaryStatisticsResponse {
        items: vec![SalaryStatisticsItem {
            category: Some("Developer".to_string()),
            salary: Some(SalaryStatValue {
                min: Some(100_000),
                max: Some(500_000),
                avg: Some(250_000),
            }),
            percentiles: Some(Percentiles {
                p10: Some(120_000),
                p25: Some(150_000),
                p50: Some(200_000),
                p75: Some(300_000),
                p90: Some(450_000),
            }),
        }],
    };

    Mock::given(method("GET"))
        .and(path("/salary_statistics"))
        .and(query_param("professional_role", "96"))
        .and(query_param("area", "1"))
        .and(query_param("currency", "RUR"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
        .mount(&server)
        .await;

    let result = client
        .get_salary_statistics("96", "1", "RUR")
        .await
        .unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].category.as_deref(), Some("Developer"));
    let salary = result.items[0].salary.as_ref().unwrap();
    assert_eq!(salary.avg, Some(250_000));
}
