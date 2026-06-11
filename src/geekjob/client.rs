//! HTTP client for GeekJob's public vacancy pages.

use std::time::Duration;

use reqwest::{Client, ClientBuilder};
use tracing::{instrument, trace};

use crate::error::{JobsmithError, Result};
use crate::geekjob::{detail, listing};
use crate::hh::models::{Vacancy, VacancyDetail};
use crate::http::{is_retryable, retry_delay, MAX_RETRIES};

/// Base URL for GeekJob.
const GEEKJOB_BASE: &str = "https://geekjob.ru";

/// Polite User-Agent with a contact, mirroring the other clients.
const GEEKJOB_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_REPOSITORY"),
    ")"
);

/// Default request timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// HTTP client for GeekJob.
#[derive(Debug, Clone)]
pub struct GeekjobClient {
    client: Client,
    base_url: String,
}

impl GeekjobClient {
    /// Create a new client with default settings.
    pub fn new() -> Result<Self> {
        Self::build(GEEKJOB_BASE.to_string())
    }

    /// Create a client with a custom base URL (useful for testing).
    pub fn with_base_url(base_url: impl Into<String>) -> Result<Self> {
        Self::build(base_url.into())
    }

    fn build(base_url: String) -> Result<Self> {
        let client = ClientBuilder::new()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(GEEKJOB_USER_AGENT)
            .build()
            .map_err(|e| JobsmithError::GeekjobRequest(e.to_string()))?;
        Ok(Self { client, base_url })
    }

    /// Search vacancies via the listing page (first page only).
    #[instrument(skip(self), fields(query = %query))]
    pub async fn search(&self, query: &str) -> Result<Vec<Vacancy>> {
        let url = format!("{}/", self.base_url);
        trace!("fetching geekjob search page");
        let body = self.get_text(&url, &[("qs", query)]).await?;
        listing::parse_search_html(&body)
    }

    /// Fetch a single vacancy by its hex id (reads the page's JSON-LD).
    #[instrument(skip(self), fields(vacancy_id = %id))]
    pub async fn get_vacancy(&self, id: &str) -> Result<VacancyDetail> {
        let url = format!("{}/vacancy/{}", self.base_url, id);
        trace!("fetching geekjob vacancy page");
        let body = self.get_text(&url, &[]).await?;
        detail::parse_vacancy_html(&body, id)
    }

    /// GET with exponential backoff retry on network errors and 429/500/503.
    async fn get_text(&self, url: &str, params: &[(&str, &str)]) -> Result<String> {
        let mut last_error: Option<JobsmithError> = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                tokio::time::sleep(retry_delay(attempt, None)).await;
            }

            match self.client.get(url).query(params).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        return response
                            .text()
                            .await
                            .map_err(|e| JobsmithError::GeekjobRequest(e.to_string()));
                    }
                    if is_retryable(status) {
                        last_error = Some(JobsmithError::GeekjobRequest(format!(
                            "server returned status {}",
                            status.as_u16()
                        )));
                        continue;
                    }
                    return Err(JobsmithError::GeekjobRequest(format!(
                        "status {}",
                        status.as_u16()
                    )));
                }
                Err(e) => {
                    last_error = Some(JobsmithError::GeekjobRequest(e.to_string()));
                    continue;
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| JobsmithError::GeekjobRequest("max retries exceeded".to_string())))
    }
}
