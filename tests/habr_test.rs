use jobsmith::error::JobsmithError;
use jobsmith::habr::HabrClient;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SEARCH_RSS: &str = include_str!("fixtures/habr_search.rss");
const VACANCY_HTML: &str = include_str!("fixtures/habr_vacancy.html");

#[tokio::test]
async fn search_parses_rss_feed() {
    let server = MockServer::start().await;
    let client = HabrClient::with_base_url(server.uri()).unwrap();

    Mock::given(method("GET"))
        .and(path("/vacancies/rss"))
        .and(query_param("q", "rust"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/rss+xml")
                .set_body_string(SEARCH_RSS),
        )
        .expect(1)
        .mount(&server)
        .await;

    let items = client.search("rust").await.unwrap();
    assert_eq!(items.len(), 9);
    assert_eq!(items[0].employer_name(), "Bell Integrator");
}

#[tokio::test]
async fn get_vacancy_reads_json_ld() {
    let server = MockServer::start().await;
    let client = HabrClient::with_base_url(server.uri()).unwrap();

    Mock::given(method("GET"))
        .and(path("/vacancies/1000166679"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/html")
                .set_body_string(VACANCY_HTML),
        )
        .expect(1)
        .mount(&server)
        .await;

    let detail = client.get_vacancy("1000166679").await.unwrap();
    assert_eq!(detail.base.name, "Разработчик Rust");
    assert_eq!(detail.employer_name(), "Bell Integrator");
}

#[tokio::test]
async fn not_found_maps_to_habr_request_error() {
    let server = MockServer::start().await;
    let client = HabrClient::with_base_url(server.uri()).unwrap();

    Mock::given(method("GET"))
        .and(path("/vacancies/404"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let err = client.get_vacancy("404").await.unwrap_err();
    assert!(matches!(err, JobsmithError::HabrRequest(_)), "{err:?}");
}
