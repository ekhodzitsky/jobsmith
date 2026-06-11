//! HTTP client for the Trudvsem open-data API.

use std::time::Duration;

use reqwest::{Client, ClientBuilder};
use tracing::{instrument, trace};

use crate::error::{JobsmithError, Result};
use crate::hh::models::{Vacancy, VacancyDetail};
use crate::http::{is_retryable, retry_delay, MAX_RETRIES};
use crate::trudvsem::api;

/// Base URL of the open-data API.
const TRUDVSEM_BASE: &str = "https://opendata.trudvsem.ru/api";

/// Polite User-Agent with a contact, mirroring the other clients.
const TRUDVSEM_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_REPOSITORY"),
    ")"
);

/// Default request timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// HTTP client for Trudvsem.
#[derive(Debug, Clone)]
pub struct TrudvsemClient {
    client: Client,
    base_url: String,
}

impl TrudvsemClient {
    /// Create a new client with default settings.
    pub fn new() -> Result<Self> {
        Self::build(TRUDVSEM_BASE.to_string())
    }

    /// Create a client with a custom base URL (useful for testing).
    pub fn with_base_url(base_url: impl Into<String>) -> Result<Self> {
        Self::build(base_url.into())
    }

    fn build(base_url: String) -> Result<Self> {
        let client = ClientBuilder::new()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(TRUDVSEM_USER_AGENT)
            .build()
            .map_err(|e| JobsmithError::TrudvsemRequest(e.to_string()))?;
        Ok(Self { client, base_url })
    }

    /// Search vacancies by text.
    #[instrument(skip(self), fields(query = %query))]
    pub async fn search(&self, query: &str, limit: u32) -> Result<Vec<Vacancy>> {
        let url = format!("{}/v1/vacancies", self.base_url);
        let limit = limit.to_string();
        trace!("fetching trudvsem search");
        let body = self
            .get_text(&url, &[("text", query), ("limit", limit.as_str())])
            .await?;
        api::parse_vacancies(&body)
    }

    /// Fetch a single vacancy by its `{companyCode}/{vacancyId}` pair.
    #[instrument(skip(self))]
    pub async fn get_vacancy(&self, company_code: &str, vacancy_id: &str) -> Result<VacancyDetail> {
        let url = format!(
            "{}/v1/vacancies/vacancy/{}/{}",
            self.base_url, company_code, vacancy_id
        );
        trace!("fetching trudvsem vacancy");
        let body = self.get_text(&url, &[]).await?;
        api::parse_vacancy_detail(&body)
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
                            .map_err(|e| JobsmithError::TrudvsemRequest(e.to_string()));
                    }
                    if is_retryable(status) {
                        last_error = Some(JobsmithError::TrudvsemRequest(format!(
                            "server returned status {}",
                            status.as_u16()
                        )));
                        continue;
                    }
                    return Err(JobsmithError::TrudvsemRequest(format!(
                        "status {}",
                        status.as_u16()
                    )));
                }
                Err(e) => {
                    last_error = Some(JobsmithError::TrudvsemRequest(e.to_string()));
                    continue;
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| JobsmithError::TrudvsemRequest("max retries exceeded".to_string())))
    }
}
