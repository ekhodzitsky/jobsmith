use jobsmith::error::JobsmithError;
use jobsmith::trudvsem::TrudvsemClient;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SEARCH: &str = include_str!("fixtures/trudvsem_search.json");
const DETAIL: &str = include_str!("fixtures/trudvsem_vacancy.json");

#[tokio::test]
async fn search_parses_open_data_response() {
    let server = MockServer::start().await;
    let client = TrudvsemClient::with_base_url(server.uri()).unwrap();

    Mock::given(method("GET"))
        .and(path("/v1/vacancies"))
        .and(query_param("text", "rust"))
        .and(query_param("limit", "5"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_string(SEARCH),
        )
        .expect(1)
        .mount(&server)
        .await;

    let items = client.search("rust", 5).await.unwrap();
    assert_eq!(items.len(), 4);
    assert_eq!(items[0].employer_name(), "МТС");
    assert!(items[0].salary.is_some(), "salary must map through");
}

#[tokio::test]
async fn get_vacancy_uses_company_and_vacancy_ids() {
    let server = MockServer::start().await;
    let client = TrudvsemClient::with_base_url(server.uri()).unwrap();

    Mock::given(method("GET"))
        .and(path(
            "/v1/vacancies/vacancy/7226c750-02f1-11eb-8600-bfd13399602c/58c3f1e4-5b4a-11f1-b964-dd82fad7662e",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_string(DETAIL),
        )
        .expect(1)
        .mount(&server)
        .await;

    let detail = client
        .get_vacancy(
            "7226c750-02f1-11eb-8600-bfd13399602c",
            "58c3f1e4-5b4a-11f1-b964-dd82fad7662e",
        )
        .await
        .unwrap();
    assert!(detail.base.name.contains("Rust TechLead"));
}

#[tokio::test]
async fn non_success_maps_to_trudvsem_error() {
    let server = MockServer::start().await;
    let client = TrudvsemClient::with_base_url(server.uri()).unwrap();

    Mock::given(method("GET"))
        .and(path("/v1/vacancies/vacancy/a/b"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let err = client.get_vacancy("a", "b").await.unwrap_err();
    assert!(matches!(err, JobsmithError::TrudvsemRequest(_)), "{err:?}");
}

/// Live smoke against the real open-data API. Network-dependent, so
/// ignored by default: `cargo test --test trudvsem_test live_ -- --ignored`.
#[ignore = "hits the live opendata.trudvsem.ru"]
#[tokio::test]
async fn live_search_roundtrip() {
    let client = TrudvsemClient::new().unwrap();
    let items = client.search("rust", 5).await.expect("live search");
    assert!(!items.is_empty(), "expected live vacancies");
    assert!(items[0].description.is_some());
}
