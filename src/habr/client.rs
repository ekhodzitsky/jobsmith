//! HTTP client for Habr Career's public vacancy channels.
//!
//! Reads the RSS search feed and the public vacancy pages — both
//! permitted by `robots.txt` — and never bypasses any access control.

use std::time::Duration;

use reqwest::{Client, ClientBuilder};
use tracing::{instrument, trace};

use crate::error::{JobsmithError, Result};
use crate::habr::{jsonld, rss};
use crate::hh::models::{Vacancy, VacancyDetail};
use crate::http::{is_retryable, retry_delay, MAX_RETRIES};

/// Base URL for Habr Career.
const HABR_BASE: &str = "https://career.habr.com";

/// Polite User-Agent with a contact, mirroring the HH client.
const HABR_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_REPOSITORY"),
    ")"
);

/// Default request timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// HTTP client for Habr Career.
#[derive(Debug, Clone)]
pub struct HabrClient {
    client: Client,
    base_url: String,
}

impl HabrClient {
    /// Create a new Habr Career client with default settings.
    pub fn new() -> Result<Self> {
        Self::build(HABR_BASE.to_string())
    }

    /// Create a client with a custom base URL (useful for testing).
    pub fn with_base_url(base_url: impl Into<String>) -> Result<Self> {
        Self::build(base_url.into())
    }

    fn build(base_url: String) -> Result<Self> {
        let client = ClientBuilder::new()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(HABR_USER_AGENT)
            .build()
            .map_err(|e| JobsmithError::HabrRequest(e.to_string()))?;
        Ok(Self { client, base_url })
    }

    /// Search vacancies via the RSS feed.
    #[instrument(skip(self), fields(query = %query))]
    pub async fn search(&self, query: &str) -> Result<Vec<Vacancy>> {
        let url = format!("{}/vacancies/rss", self.base_url);
        trace!("fetching habr rss search feed");
        let response = self.send_with_retry(&url, &[("q", query)]).await?;
        let body = response
            .text()
            .await
            .map_err(|e| JobsmithError::HabrRequest(e.to_string()))?;
        rss::parse_search_rss(&body)
    }

    /// Fetch a single vacancy by id (reads the page's JSON-LD).
    #[instrument(skip(self), fields(vacancy_id = %id))]
    pub async fn get_vacancy(&self, id: &str) -> Result<VacancyDetail> {
        let url = format!("{}/vacancies/{}", self.base_url, id);
        trace!("fetching habr vacancy page");
        let response = self.send_with_retry(&url, &[]).await?;
        let body = response
            .text()
            .await
            .map_err(|e| JobsmithError::HabrRequest(e.to_string()))?;
        jsonld::parse_vacancy_html(&body, id)
    }

    /// GET with exponential backoff retry on network errors and 429/500/503.
    async fn send_with_retry(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> Result<reqwest::Response> {
        let mut last_error: Option<JobsmithError> = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                tokio::time::sleep(retry_delay(attempt, None)).await;
            }

            match self.client.get(url).query(params).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        return Ok(response);
                    }
                    if is_retryable(status) {
                        last_error = Some(JobsmithError::HabrRequest(format!(
                            "server returned status {}",
                            status.as_u16()
                        )));
                        continue;
                    }
                    return Err(JobsmithError::HabrRequest(format!(
                        "status {}",
                        status.as_u16()
                    )));
                }
                Err(e) => {
                    last_error = Some(JobsmithError::HabrRequest(e.to_string()));
                    continue;
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| JobsmithError::HabrRequest("max retries exceeded".to_string())))
    }
}
