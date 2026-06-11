use jobsmith::error::JobsmithError;
use jobsmith::geekjob::GeekjobClient;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SEARCH: &str = include_str!("fixtures/geekjob_search.html");
const VACANCY: &str = include_str!("fixtures/geekjob_vacancy_salary.html");

#[tokio::test]
async fn search_parses_listing_page() {
    let server = MockServer::start().await;
    let client = GeekjobClient::with_base_url(server.uri()).unwrap();

    Mock::given(method("GET"))
        .and(path("/"))
        .and(query_param("qs", "rust"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/html")
                .set_body_string(SEARCH),
        )
        .expect(1)
        .mount(&server)
        .await;

    let items = client.search("rust").await.unwrap();
    assert_eq!(items.len(), 14);
}

#[tokio::test]
async fn get_vacancy_reads_json_ld_with_salary() {
    let server = MockServer::start().await;
    let client = GeekjobClient::with_base_url(server.uri()).unwrap();

    Mock::given(method("GET"))
        .and(path("/vacancy/691aef77ce60ad56c3046275"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/html")
                .set_body_string(VACANCY),
        )
        .expect(1)
        .mount(&server)
        .await;

    let detail = client
        .get_vacancy("691aef77ce60ad56c3046275")
        .await
        .unwrap();
    assert!(detail.base.name.contains("QA automation"));
    assert_eq!(
        detail.base.salary.as_ref().and_then(|s| s.from),
        Some(4_000)
    );
}

#[tokio::test]
async fn not_found_maps_to_geekjob_error() {
    let server = MockServer::start().await;
    let client = GeekjobClient::with_base_url(server.uri()).unwrap();

    Mock::given(method("GET"))
        .and(path("/vacancy/00000000000000000000dead"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let err = client
        .get_vacancy("00000000000000000000dead")
        .await
        .unwrap_err();
    assert!(matches!(err, JobsmithError::GeekjobRequest(_)), "{err:?}");
}

/// Live smoke against the real GeekJob site. Network-dependent, so
/// ignored by default: `cargo test --test geekjob_test live_ -- --ignored`.
#[ignore = "hits the live geekjob.ru"]
#[tokio::test]
async fn live_search_and_detail_roundtrip() {
    let client = GeekjobClient::new().unwrap();
    let items = client.search("rust").await.expect("live search");
    assert!(!items.is_empty(), "expected live vacancies");

    let detail = client.get_vacancy(&items[0].id).await.expect("live detail");
    assert!(!detail.base.name.is_empty());
    assert!(detail.base.description.is_some());
}
