use jobsmith::commands::salary_cmd;
use jobsmith::error::JobsmithError;
use jobsmith::hh::HhClient;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn run_online_prints_statistics_for_role_and_area() {
    let server = MockServer::start().await;
    let client = HhClient::with_base_url(server.uri()).unwrap();

    Mock::given(method("GET"))
        .and(path("/salary_statistics"))
        .and(query_param("professional_role", "96"))
        .and(query_param("area", "1"))
        .and(query_param("currency", "RUR"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [{
                "category": "Разработчик",
                "salary": {"min": 150_000, "max": 400_000, "avg": 250_000},
                "percentiles": {"25": 180_000, "50": 250_000, "75": 320_000}
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    salary_cmd::run_online(&client, "96", "1", "RUR", false)
        .await
        .unwrap();
}

#[tokio::test]
async fn run_online_maps_empty_items_to_salary_not_found() {
    let server = MockServer::start().await;
    let client = HhClient::with_base_url(server.uri()).unwrap();

    Mock::given(method("GET"))
        .and(path("/salary_statistics"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"items": []})))
        .mount(&server)
        .await;

    let err = salary_cmd::run_online(&client, "96", "1", "RUR", false)
        .await
        .unwrap_err();
    assert!(
        matches!(err, JobsmithError::SalaryNotFound { .. }),
        "expected SalaryNotFound, got {err:?}"
    );
}
