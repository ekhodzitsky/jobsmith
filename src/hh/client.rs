//! HeadHunter API HTTP client.

use std::time::Duration;

use reqwest::{Client, ClientBuilder};
use tracing::{instrument, trace};

use crate::error::{JobsmithError, Result};
use crate::hh::models::{
    SalaryStatisticsResponse, VacanciesResponse, VacancyDetail, VacancySearchQuery,
};
use crate::http::{is_retryable, parse_retry_after, retry_delay, MAX_RETRIES};

/// Base URL for the HeadHunter API.
const HH_API_BASE: &str = "https://api.hh.ru";

/// User-Agent required by HH API terms of service.
const HH_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_REPOSITORY"),
    ")"
);

/// Default timeout for HH API requests.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// HTTP client for the HeadHunter API.
#[derive(Debug, Clone)]
pub struct HhClient {
    client: Client,
    base_url: String,
}

impl HhClient {
    /// Create a new HH API client with default settings.
    pub fn new() -> Result<Self> {
        let client = ClientBuilder::new()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(HH_USER_AGENT)
            .build()
            .map_err(JobsmithError::HhApiRequest)?;

        Ok(Self {
            client,
            base_url: HH_API_BASE.to_string(),
        })
    }

    /// Create a client with a custom base URL (useful for testing).
    pub fn with_base_url(base_url: impl Into<String>) -> Result<Self> {
        let client = ClientBuilder::new()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(HH_USER_AGENT)
            .build()
            .map_err(JobsmithError::HhApiRequest)?;

        Ok(Self {
            client,
            base_url: base_url.into(),
        })
    }

    /// Send a request with exponential backoff retry.
    ///
    /// Retries on network errors (`reqwest::Error`), HTTP 429, 500, and 503.
    /// Backoff: ~1s → ~2s → ~4s for up to 3 retries (with up to 500 ms jitter).
    async fn send_with_retry(
        &self,
        method: reqwest::Method,
        url: &str,
        params: Option<&[(&str, &str)]>,
    ) -> Result<reqwest::Response> {
        let mut last_error: Option<JobsmithError> = None;
        let mut retry_after: Option<Duration> = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                tokio::time::sleep(retry_delay(attempt, retry_after.take())).await;
            }

            let mut request = self.client.request(method.clone(), url);
            if let Some(p) = params {
                request = request.query(p);
            }

            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        return Ok(response);
                    }
                    if is_retryable(status) {
                        retry_after = parse_retry_after(&response);
                        last_error = Some(JobsmithError::HhApiStatus {
                            status: status.as_u16(),
                            message: "server indicated retry".to_string(),
                        });
                        continue;
                    }
                    let message = match response.text().await {
                        Ok(text) if !text.trim().is_empty() => text,
                        Ok(_) => "empty error response body".to_string(),
                        Err(e) => {
                            return Err(JobsmithError::HhApiRequest(e));
                        }
                    };
                    return Err(JobsmithError::HhApiStatus {
                        status: status.as_u16(),
                        message,
                    });
                }
                Err(e) => {
                    last_error = Some(JobsmithError::HhApiRequest(e));
                    continue;
                }
            }
        }

        match last_error {
            Some(e) => Err(e),
            None => Err(JobsmithError::HhApiStatus {
                status: 0,
                message: "max retries exceeded".to_string(),
            }),
        }
    }

    /// Search for vacancies with the given query.
    #[instrument(skip(self), fields(query.text = ?query.text))]
    pub async fn search_vacancies(&self, query: &VacancySearchQuery) -> Result<VacanciesResponse> {
        let url = format!("{}/vacancies", self.base_url);
        let params = query.to_params();
        let param_refs: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        trace!("sending hh api request");
        let response = self
            .send_with_retry(reqwest::Method::GET, &url, Some(&param_refs))
            .await?;

        let vacancies = response.json().await.map_err(JobsmithError::HhApiRequest)?;
        Ok(vacancies)
    }

    /// Fetch a single vacancy by ID.
    #[instrument(skip(self), fields(vacancy_id = %id))]
    pub async fn get_vacancy(&self, id: &str) -> Result<VacancyDetail> {
        let url = format!("{}/vacancies/{}", self.base_url, id);

        trace!("sending hh api request");
        let response = self
            .send_with_retry(reqwest::Method::GET, &url, None)
            .await?;

        let vacancy = response.json().await.map_err(JobsmithError::HhApiRequest)?;
        Ok(vacancy)
    }

    /// Fetch salary statistics for a professional role in a given area.
    ///
    /// Not wired into a command yet: kept (and tested) as the intended
    /// online data source for `jobsmith salary` alongside the local
    /// `salary_data.json` lookup.
    #[instrument(skip(self), fields(professional_role = %professional_role, area = %area))]
    pub async fn get_salary_statistics(
        &self,
        professional_role: &str,
        area: &str,
        currency: &str,
    ) -> Result<SalaryStatisticsResponse> {
        let url = format!("{}/salary_statistics", self.base_url);
        let params = [
            ("professional_role", professional_role),
            ("area", area),
            ("currency", currency),
        ];

        trace!("sending hh api request");
        let response = self
            .send_with_retry(reqwest::Method::GET, &url, Some(&params))
            .await?;

        let stats = response.json().await.map_err(JobsmithError::HhApiRequest)?;
        Ok(stats)
    }
}

/// Extract a numeric vacancy ID from a raw input.
///
/// Accepts either a plain ID (`"123456"`) or an HH vacancy URL
/// (`"https://hh.ru/vacancy/123456"`). Returns an error if the
/// extracted ID is not numeric.
pub fn extract_vacancy_id(input: &str) -> Result<String> {
    let id = if input.chars().all(|c| c.is_ascii_digit()) {
        input.to_string()
    } else {
        match input
            .split('/')
            .rev()
            .find(|s| !s.is_empty())
            .and_then(|s| s.split('?').next())
            .and_then(|s| s.split('#').next())
        {
            Some(segment) => segment.to_string(),
            None => input.to_string(),
        }
    };

    if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
        Ok(id)
    } else {
        Err(JobsmithError::InvalidVacancyId(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_delay_honours_longer_retry_after() {
        assert!(retry_delay(1, Some(Duration::from_secs(7))) >= Duration::from_secs(7));
    }

    #[test]
    fn retry_delay_keeps_backoff_when_retry_after_is_shorter() {
        // backoff_with_jitter(1) is at least 1s, (2) at least 2s
        assert!(retry_delay(1, Some(Duration::from_millis(1))) >= Duration::from_secs(1));
        assert!(retry_delay(2, None) >= Duration::from_secs(2));
    }
}
