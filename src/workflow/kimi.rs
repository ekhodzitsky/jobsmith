//! Kimi Code wire protocol client wrapper.
//!
//! Wraps `kimi-wire` transport and provides high-level workflow methods.

use std::time::Duration;

use kimi_wire::client::WireClient;
use kimi_wire::client_ext::RequestExt;
use kimi_wire::message::{parse_wire_message, WireMessage};
use kimi_wire::protocol::{ContentPart, Event, PromptResult, TextPart, ThinkPart};
use kimi_wire::transport::{ChildProcessTransport, TransportWireClient};
use tracing::{debug, info, instrument, warn};

use crate::error::{JobsmithError, Result};
use crate::hh::models::VacancyDetail;
use crate::profile::model::Profile;
use crate::workflow::parser;
use crate::workflow::prompts;
use crate::workflow::state::FitScore;

const DEFAULT_PROMPT_TIMEOUT: Duration = Duration::from_secs(300);
/// Bound on spawning `kimi --wire` plus the initialize handshake;
/// a hung handshake otherwise blocks `apply` forever (AGENTS.md rule 6).
const SPAWN_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024; // 10 MiB

/// Result of a single prompt turn.
#[derive(Debug, Clone)]
pub struct KimiPromptResult {
    /// Turn completion status.
    pub status: kimi_wire::protocol::PromptStatus,
    /// Collected text output from the turn.
    pub turn_output: String,
}

/// High-level client for Kimi Code wire protocol.
#[derive(Debug)]
pub struct KimiClient {
    inner: TransportWireClient<ChildProcessTransport>,
    timeout: Duration,
}

impl KimiClient {
    /// Spawn a new Kimi client via `kimi --wire`.
    #[instrument]
    pub async fn spawn() -> Result<Self> {
        Self::spawn_binary_with_timeout("kimi", SPAWN_TIMEOUT).await
    }

    /// Spawn a wire client from an explicit binary, bounding both the
    /// process spawn and the protocol handshake by `spawn_timeout`.
    ///
    /// On timeout the transport is dropped, and `kill_on_drop` inside
    /// `ChildProcessTransport` reaps the child.
    async fn spawn_binary_with_timeout(binary: &str, spawn_timeout: Duration) -> Result<Self> {
        tokio::time::timeout(spawn_timeout, async {
            let transport = ChildProcessTransport::spawn(binary, None, None, None)
                .await
                .map_err(|e| JobsmithError::KimiWire(e.to_string()))?;
            let mut client = Self {
                inner: TransportWireClient::new(transport),
                timeout: DEFAULT_PROMPT_TIMEOUT,
            };
            client.initialize().await?;
            Ok(client)
        })
        .await
        .map_err(|_| JobsmithError::ProcessTimeout {
            duration_secs: spawn_timeout.as_secs(),
        })?
    }

    /// Initialize the wire protocol handshake.
    async fn initialize(&mut self) -> Result<()> {
        let params = kimi_wire::protocol::InitializeParams::new(kimi_wire::WIRE_PROTOCOL_VERSION);
        self.inner
            .initialize(params)
            .await
            .map_err(|e| JobsmithError::KimiWire(e.to_string()))?;
        info!("kimi wire initialized");
        Ok(())
    }

    /// Set a custom prompt timeout.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Send a prompt and collect the text output.
    async fn prompt_and_collect(
        &mut self,
        text: impl Into<kimi_wire::protocol::UserInput> + Send,
    ) -> Result<KimiPromptResult> {
        let id = self
            .inner
            .start_prompt(text)
            .await
            .map_err(|e| JobsmithError::KimiWire(e.to_string()))?;

        let mut output = String::new();
        let deadline = tokio::time::Instant::now() + self.timeout;

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            let raw = tokio::time::timeout(remaining, self.inner.read_raw_message())
                .await
                .map_err(|_| JobsmithError::ProcessTimeout {
                    duration_secs: self.timeout.as_secs(),
                })?
                .map_err(|e| JobsmithError::KimiWire(e.to_string()))?;

            let msg =
                parse_wire_message(raw).map_err(|e| JobsmithError::KimiWire(e.to_string()))?;

            match msg {
                WireMessage::Event(notification) => {
                    if let Event::ContentPart(part) = notification.params {
                        match part {
                            ContentPart::Text(TextPart { text }) => {
                                output.push_str(&text);
                                if output.len() > MAX_OUTPUT_BYTES {
                                    return Err(JobsmithError::Process(
                                        "kimi output exceeded 10 MiB limit".to_string(),
                                    ));
                                }
                            }
                            ContentPart::Think(ThinkPart { think, .. }) => {
                                debug!(think_len = think.len(), "received thinking content");
                            }
                            other => {
                                debug!(content_part = ?other, "received non-text content part");
                            }
                        }
                    }
                }
                WireMessage::Request(req) => {
                    let response = req.params.default_response();
                    self.inner
                        .send_response(&req.id, response)
                        .await
                        .map_err(|e| JobsmithError::KimiWire(e.to_string()))?;
                }
                WireMessage::SuccessResponse(resp) if resp.id == id => {
                    let result: PromptResult = serde_json::from_value(resp.result)
                        .map_err(|e| JobsmithError::KimiWire(e.to_string()))?;
                    return Ok(KimiPromptResult {
                        status: result.status,
                        turn_output: output,
                    });
                }
                WireMessage::ErrorResponse(resp) if resp.id == id => {
                    return Err(JobsmithError::KimiWire(format!(
                        "prompt failed: {} (code: {})",
                        resp.error.message, resp.error.code
                    )));
                }
                WireMessage::SuccessResponse(resp) => {
                    warn!(response_id = %resp.id, expected_id = %id, "unexpected success response id");
                }
                WireMessage::ErrorResponse(resp) => {
                    warn!(response_id = %resp.id, expected_id = %id, "unexpected error response id");
                }
            }
        }
    }

    /// Evaluate how well the profile fits the vacancy.
    #[instrument(skip(self, profile, vacancy))]
    pub async fn evaluate_fit(
        &mut self,
        profile: &Profile,
        vacancy: &VacancyDetail,
    ) -> Result<(FitScore, String)> {
        let prompt = prompts::build_fit_evaluation_prompt(profile, vacancy);
        let result = self.prompt_and_collect(prompt).await?;
        let evaluation = parser::parse_evaluation(&result.turn_output)?;
        Ok((FitScore::new(evaluation.score)?, result.turn_output))
    }

    /// Draft CV and cover letter.
    #[instrument(skip(self, profile, vacancy, evaluation))]
    pub async fn draft_cv(
        &mut self,
        profile: &Profile,
        vacancy: &VacancyDetail,
        evaluation: &str,
    ) -> Result<(String, String)> {
        let cv_prompt = prompts::build_cv_draft_prompt(profile, vacancy, evaluation);
        let cv_result = self.prompt_and_collect(cv_prompt).await?;

        let cover_prompt =
            prompts::build_cover_draft_prompt(profile, vacancy, &cv_result.turn_output);
        let cover_result = self.prompt_and_collect(cover_prompt).await?;

        Ok((cv_result.turn_output, cover_result.turn_output))
    }

    /// Review the drafted documents.
    #[instrument(skip(self, profile, vacancy, cv_draft, cover_draft))]
    pub async fn review(
        &mut self,
        profile: &Profile,
        vacancy: &VacancyDetail,
        cv_draft: &str,
        cover_draft: &str,
    ) -> Result<String> {
        let prompt = prompts::build_reviewer_prompt(profile, vacancy, cv_draft, cover_draft);
        let result = self.prompt_and_collect(prompt).await?;
        Ok(result.turn_output)
    }

    /// Revise documents based on review feedback.
    #[instrument(skip(self, profile, cv_draft, cover_draft, review))]
    pub async fn revise(
        &mut self,
        profile: &Profile,
        cv_draft: &str,
        cover_draft: &str,
        review: &str,
    ) -> Result<(String, String)> {
        let prompt = prompts::build_revision_prompt(profile, cv_draft, cover_draft, review);
        let result = self.prompt_and_collect(prompt).await?;
        parser::parse_revised(&result.turn_output)
    }

    /// Gracefully shut down the client.
    pub async fn shutdown(self) -> Result<()> {
        self.inner
            .shutdown()
            .await
            .map_err(|e| JobsmithError::KimiWire(e.to_string()))?;
        info!("kimi client shut down");
        Ok(())
    }
}

#[allow(clippy::manual_async_fn)]
impl crate::workflow::client::WorkflowClient for KimiClient {
    fn evaluate_fit<'a>(
        &'a mut self,
        profile: &'a Profile,
        vacancy: &'a VacancyDetail,
    ) -> impl std::future::Future<Output = Result<(FitScore, String)>> + 'a {
        async move { KimiClient::evaluate_fit(self, profile, vacancy).await }
    }

    fn draft_cv<'a>(
        &'a mut self,
        profile: &'a Profile,
        vacancy: &'a VacancyDetail,
        evaluation_text: &'a str,
    ) -> impl std::future::Future<Output = Result<(String, String)>> + 'a {
        async move { KimiClient::draft_cv(self, profile, vacancy, evaluation_text).await }
    }

    fn review<'a>(
        &'a mut self,
        profile: &'a Profile,
        vacancy: &'a VacancyDetail,
        cv_draft: &'a str,
        cover_draft: &'a str,
    ) -> impl std::future::Future<Output = Result<String>> + 'a {
        async move { KimiClient::review(self, profile, vacancy, cv_draft, cover_draft).await }
    }

    fn revise<'a>(
        &'a mut self,
        profile: &'a Profile,
        cv_draft: &'a str,
        cover_draft: &'a str,
        review: &'a str,
    ) -> impl std::future::Future<Output = Result<(String, String)>> + 'a {
        async move { KimiClient::revise(self, profile, cv_draft, cover_draft, review).await }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A spawn against a process that never completes the handshake must
    /// fail with `ProcessTimeout` in bounded time instead of hanging.
    #[cfg(unix)]
    #[tokio::test]
    async fn spawn_times_out_on_hung_handshake() {
        use std::os::unix::fs::PermissionsExt;

        let script =
            std::env::temp_dir().join(format!("jobsmith-hung-kimi-{}.sh", std::process::id()));
        std::fs::write(&script, "#!/bin/sh\nsleep 600\n").unwrap();
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).unwrap();

        let result = tokio::time::timeout(
            Duration::from_secs(5),
            KimiClient::spawn_binary_with_timeout(
                script.to_str().unwrap_or_default(),
                Duration::from_millis(300),
            ),
        )
        .await;
        // best-effort cleanup of the helper script
        let _ = std::fs::remove_file(&script);

        let spawn_result = result.expect("spawn must finish in bounded time, not hang");
        match spawn_result {
            Err(JobsmithError::ProcessTimeout { .. }) => {}
            other => panic!("expected ProcessTimeout, got {other:?}"),
        }
    }
}
