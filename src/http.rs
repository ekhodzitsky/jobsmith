//! Shared HTTP helpers (retry backoff) used by the API clients.

use std::time::Duration;

/// Maximum number of retries for transient upstream failures.
pub(crate) const MAX_RETRIES: u32 = 3;

/// Compute a delay with jitter for retry backoff.
///
/// Uses the current system time nanoseconds as a lightweight entropy
/// source to avoid a `rand` dependency while still preventing
/// thundering-herd synchronization across independent processes.
pub(crate) fn backoff_with_jitter(attempt: u32) -> Duration {
    let base_secs = 2u64.saturating_pow(attempt.saturating_sub(1));
    let base = Duration::from_secs(base_secs);
    let jitter_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
        % 500;
    base + Duration::from_millis(jitter_ms)
}

/// Parse a `Retry-After` header (delay-seconds form; HTTP-date is ignored).
pub(crate) fn parse_retry_after(response: &reqwest::Response) -> Option<Duration> {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}

/// Delay before retry `attempt`; a server-provided `Retry-After` wins
/// when it is longer than the local backoff.
pub(crate) fn retry_delay(attempt: u32, retry_after: Option<Duration>) -> Duration {
    let backoff = backoff_with_jitter(attempt);
    retry_after.map_or(backoff, |ra| ra.max(backoff))
}

/// True for statuses worth retrying (429, 500, 503).
pub(crate) fn is_retryable(status: reqwest::StatusCode) -> bool {
    matches!(
        status,
        reqwest::StatusCode::TOO_MANY_REQUESTS
            | reqwest::StatusCode::INTERNAL_SERVER_ERROR
            | reqwest::StatusCode::SERVICE_UNAVAILABLE
    )
}
