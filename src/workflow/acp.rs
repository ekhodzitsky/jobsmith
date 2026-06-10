//! ACP (Agent Client Protocol) client for Kimi Code.
//!
//! Speaks line-delimited JSON-RPC over the stdio of `kimi acp` and
//! provides the high-level workflow methods. The protocol subset used
//! here (initialize, session/new, session/prompt, session/update,
//! session/request_permission) was captured against Kimi Code CLI
//! 0.14 / ACP protocol version 1.

use std::process::Stdio;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tracing::{debug, info, instrument, warn};

use crate::error::{JobsmithError, Result};
use crate::hh::models::VacancyDetail;
use crate::profile::model::Profile;
use crate::workflow::parser;
use crate::workflow::prompts;
use crate::workflow::state::FitScore;

const DEFAULT_PROMPT_TIMEOUT: Duration = Duration::from_secs(300);
/// Bound on spawning `kimi acp` plus the initialize and session/new
/// handshake; a hung agent otherwise blocks `apply` forever.
const SPAWN_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024; // 10 MiB
const ACP_PROTOCOL_VERSION: u64 = 1;

/// High-level client for Kimi Code over ACP.
#[derive(Debug)]
pub struct AcpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    session_id: String,
    next_id: u64,
    timeout: Duration,
}

impl AcpClient {
    /// Spawn a new Kimi agent via `kimi acp`.
    #[instrument]
    pub async fn spawn() -> Result<Self> {
        Self::spawn_binary_with_timeout("kimi", SPAWN_TIMEOUT).await
    }

    /// Spawn an ACP agent from an explicit binary, bounding the process
    /// spawn, the initialize handshake and session creation by
    /// `spawn_timeout`.
    ///
    /// On timeout the child is dropped, and `kill_on_drop` reaps it.
    async fn spawn_binary_with_timeout(binary: &str, spawn_timeout: Duration) -> Result<Self> {
        tokio::time::timeout(spawn_timeout, async {
            let mut child = Command::new(binary)
                .arg("acp")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .map_err(|e| JobsmithError::Process(format!("{binary} acp: {e}")))?;
            let stdin = child.stdin.take().ok_or_else(|| {
                JobsmithError::KimiProtocol("child stdin unavailable".to_string())
            })?;
            let stdout = child.stdout.take().map(BufReader::new).ok_or_else(|| {
                JobsmithError::KimiProtocol("child stdout unavailable".to_string())
            })?;

            let mut client = Self {
                child,
                stdin,
                stdout,
                session_id: String::new(),
                next_id: 0,
                timeout: DEFAULT_PROMPT_TIMEOUT,
            };
            client.initialize().await?;
            client.open_session().await?;
            Ok(client)
        })
        .await
        .map_err(|_| JobsmithError::ProcessTimeout {
            duration_secs: spawn_timeout.as_secs(),
        })?
    }

    /// Send a request and pump the stream until its response arrives.
    async fn request(&mut self, method: &str, params: Value) -> Result<(Value, String)> {
        let id = self.next_id;
        self.next_id += 1;
        send_request(&mut self.stdin, id, method, params).await?;
        pump_until_response(&mut self.stdout, &mut self.stdin, id, self.timeout).await
    }

    /// Perform the ACP initialize handshake.
    async fn initialize(&mut self) -> Result<()> {
        let (result, _) = self
            .request(
                "initialize",
                json!({
                    "protocolVersion": ACP_PROTOCOL_VERSION,
                    "clientCapabilities": {
                        "fs": {"readTextFile": false, "writeTextFile": false}
                    },
                }),
            )
            .await?;
        debug!(agent = ?result.get("agentInfo"), "acp initialized");
        Ok(())
    }

    /// Open the agent session all prompts run in.
    async fn open_session(&mut self) -> Result<()> {
        let cwd = std::env::current_dir().map_err(JobsmithError::Io)?;
        let (result, _) = self
            .request(
                "session/new",
                json!({"cwd": cwd.to_string_lossy(), "mcpServers": []}),
            )
            .await?;
        self.session_id = result
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                JobsmithError::KimiProtocol("session/new response missing sessionId".to_string())
            })?
            .to_string();
        info!(session_id = %self.session_id, "acp session opened");
        Ok(())
    }

    /// Send a prompt and collect the agent's text output for the turn.
    async fn prompt_and_collect(&mut self, text: String) -> Result<String> {
        let session_id = self.session_id.clone();
        let (result, output) = self
            .request(
                "session/prompt",
                json!({
                    "sessionId": session_id,
                    "prompt": [{"type": "text", "text": text}],
                }),
            )
            .await?;
        let stop_reason = result
            .get("stopReason")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        if stop_reason != "end_turn" {
            warn!(stop_reason, "prompt turn ended abnormally");
        }
        Ok(output)
    }

    /// Gracefully shut down the agent: close stdin, give it a moment to
    /// exit, then kill (backed by `kill_on_drop`).
    pub async fn shutdown(self) -> Result<()> {
        let Self {
            mut child, stdin, ..
        } = self;
        drop(stdin);
        match tokio::time::timeout(Duration::from_secs(2), child.wait()).await {
            Ok(status) => {
                status.map_err(JobsmithError::Io)?;
            }
            Err(_) => {
                child
                    .kill()
                    .await
                    .map_err(|e| JobsmithError::Process(format!("kimi kill: {e}")))?;
            }
        }
        info!("acp client shut down");
        Ok(())
    }

    /// Evaluate how well the profile fits the vacancy.
    #[instrument(skip(self, profile, vacancy))]
    pub async fn evaluate_fit(
        &mut self,
        profile: &Profile,
        vacancy: &VacancyDetail,
    ) -> Result<(FitScore, String)> {
        let prompt = prompts::build_fit_evaluation_prompt(profile, vacancy);
        let output = self.prompt_and_collect(prompt).await?;
        let evaluation = parser::parse_evaluation(&output)?;
        Ok((FitScore::new(evaluation.score)?, output))
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
        let cv = self.prompt_and_collect(cv_prompt).await?;

        let cover_prompt = prompts::build_cover_draft_prompt(profile, vacancy, &cv);
        let cover = self.prompt_and_collect(cover_prompt).await?;

        Ok((cv, cover))
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
        self.prompt_and_collect(prompt).await
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
        let output = self.prompt_and_collect(prompt).await?;
        parser::parse_revised(&output)
    }
}

/// Write one JSON-RPC request line.
async fn send_request<W>(writer: &mut W, id: u64, method: &str, params: Value) -> Result<()>
where
    W: AsyncWrite + Unpin + Send,
{
    let msg = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
    send_value(writer, &msg).await
}

async fn send_value<W>(writer: &mut W, msg: &Value) -> Result<()>
where
    W: AsyncWrite + Unpin + Send,
{
    let mut line = serde_json::to_string(msg).map_err(JobsmithError::Json)?;
    line.push('\n');
    writer
        .write_all(line.as_bytes())
        .await
        .map_err(JobsmithError::Io)?;
    writer.flush().await.map_err(JobsmithError::Io)
}

/// Pump the ACP stream until the response to `request_id` arrives.
///
/// Along the way: collects `agent_message_chunk` text (bounded by
/// [`MAX_OUTPUT_BYTES`]), auto-approves `session/request_permission`
/// with the first offered option, and politely rejects other agent
/// requests. Generic over the streams so tests can drive it with
/// in-memory pipes.
async fn pump_until_response<R, W>(
    reader: &mut R,
    writer: &mut W,
    request_id: u64,
    prompt_timeout: Duration,
) -> Result<(Value, String)>
where
    R: AsyncBufRead + Unpin + Send,
    W: AsyncWrite + Unpin + Send,
{
    let mut output = String::new();
    let mut line = String::new();
    let deadline = tokio::time::Instant::now() + prompt_timeout;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        line.clear();
        let read = tokio::time::timeout(remaining, reader.read_line(&mut line))
            .await
            .map_err(|_| JobsmithError::ProcessTimeout {
                duration_secs: prompt_timeout.as_secs(),
            })?
            .map_err(JobsmithError::Io)?;
        if read == 0 {
            return Err(JobsmithError::KimiProtocol(
                "acp stream closed before the response arrived".to_string(),
            ));
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let msg: Value = serde_json::from_str(trimmed)
            .map_err(|e| JobsmithError::KimiProtocol(format!("invalid acp frame: {e}")))?;

        // Response to our request?
        if msg.get("id").and_then(Value::as_u64) == Some(request_id) && msg.get("method").is_none()
        {
            if let Some(err) = msg.get("error") {
                let code = err.get("code").and_then(Value::as_i64).unwrap_or(0);
                let message = err
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error");
                return Err(JobsmithError::KimiProtocol(format!(
                    "request failed: {message} (code: {code})"
                )));
            }
            let result = msg.get("result").cloned().unwrap_or(Value::Null);
            return Ok((result, output));
        }

        match msg.get("method").and_then(Value::as_str) {
            Some("session/update") => {
                let update = &msg["params"]["update"];
                match update.get("sessionUpdate").and_then(Value::as_str) {
                    Some("agent_message_chunk") => {
                        if let Some(text) = update["content"]["text"].as_str() {
                            output.push_str(text);
                            if output.len() > MAX_OUTPUT_BYTES {
                                return Err(JobsmithError::KimiProtocol(
                                    "kimi output exceeded 10 MiB limit".to_string(),
                                ));
                            }
                        }
                    }
                    Some("agent_thought_chunk") => {
                        debug!("received thinking chunk");
                    }
                    other => {
                        debug!(update = ?other, "ignoring session update");
                    }
                }
            }
            Some("session/request_permission") if msg.get("id").is_some() => {
                let option_id = msg["params"]["options"]
                    .as_array()
                    .and_then(|opts| opts.first())
                    .and_then(|o| o.get("optionId"))
                    .cloned();
                let outcome = match option_id {
                    Some(option_id) => json!({"outcome": "selected", "optionId": option_id}),
                    None => json!({"outcome": "cancelled"}),
                };
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": msg["id"],
                    "result": {"outcome": outcome},
                });
                send_value(writer, &response).await?;
            }
            Some(method) if msg.get("id").is_some() => {
                // Unknown agent request (we declared no fs/terminal
                // capabilities): answer so the agent doesn't stall.
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": msg["id"],
                    "error": {"code": -32601, "message": format!("method not supported: {method}")},
                });
                send_value(writer, &response).await?;
            }
            other => {
                debug!(method = ?other, "ignoring acp frame");
            }
        }
    }
}

#[allow(clippy::manual_async_fn)]
impl crate::workflow::client::WorkflowClient for AcpClient {
    fn evaluate_fit<'a>(
        &'a mut self,
        profile: &'a Profile,
        vacancy: &'a VacancyDetail,
    ) -> impl std::future::Future<Output = Result<(FitScore, String)>> + 'a {
        async move { AcpClient::evaluate_fit(self, profile, vacancy).await }
    }

    fn draft_cv<'a>(
        &'a mut self,
        profile: &'a Profile,
        vacancy: &'a VacancyDetail,
        evaluation_text: &'a str,
    ) -> impl std::future::Future<Output = Result<(String, String)>> + 'a {
        async move { AcpClient::draft_cv(self, profile, vacancy, evaluation_text).await }
    }

    fn review<'a>(
        &'a mut self,
        profile: &'a Profile,
        vacancy: &'a VacancyDetail,
        cv_draft: &'a str,
        cover_draft: &'a str,
    ) -> impl std::future::Future<Output = Result<String>> + 'a {
        async move { AcpClient::review(self, profile, vacancy, cv_draft, cover_draft).await }
    }

    fn revise<'a>(
        &'a mut self,
        profile: &'a Profile,
        cv_draft: &'a str,
        cover_draft: &'a str,
        review: &'a str,
    ) -> impl std::future::Future<Output = Result<(String, String)>> + 'a {
        async move { AcpClient::revise(self, profile, cv_draft, cover_draft, review).await }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn frames(lines: &[&str]) -> Cursor<Vec<u8>> {
        Cursor::new(format!("{}\n", lines.join("\n")).into_bytes())
    }

    #[tokio::test]
    async fn pump_collects_message_chunks_until_response() {
        let mut reader = frames(&[
            r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"available_commands_update","availableCommands":[]}}}"#,
            r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"agent_thought_chunk","content":{"type":"text","text":"thinking"}}}}"#,
            r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"Hello, "}}}}"#,
            r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"world"}}}}"#,
            r#"{"jsonrpc":"2.0","id":2,"result":{"stopReason":"end_turn"}}"#,
        ]);
        let mut writer = Vec::new();

        let (result, output) =
            pump_until_response(&mut reader, &mut writer, 2, Duration::from_secs(5))
                .await
                .unwrap();

        assert_eq!(output, "Hello, world");
        assert_eq!(
            result.get("stopReason").and_then(Value::as_str),
            Some("end_turn")
        );
        assert!(writer.is_empty(), "no agent requests, nothing to answer");
    }

    #[tokio::test]
    async fn pump_auto_approves_permission_requests() {
        let mut reader = frames(&[
            r#"{"jsonrpc":"2.0","id":7,"method":"session/request_permission","params":{"sessionId":"s","options":[{"optionId":"allow-once","name":"Allow","kind":"allow_once"}]}}"#,
            r#"{"jsonrpc":"2.0","id":3,"result":{"stopReason":"end_turn"}}"#,
        ]);
        let mut writer = Vec::new();

        pump_until_response(&mut reader, &mut writer, 3, Duration::from_secs(5))
            .await
            .unwrap();

        let sent = String::from_utf8(writer).unwrap();
        assert!(sent.contains("\"id\":7"), "{sent}");
        assert!(sent.contains("allow-once"), "{sent}");
        assert!(sent.contains("\"outcome\":\"selected\""), "{sent}");
    }

    #[tokio::test]
    async fn pump_rejects_unknown_agent_requests() {
        let mut reader = frames(&[
            r#"{"jsonrpc":"2.0","id":9,"method":"fs/read_text_file","params":{"path":"/etc/passwd"}}"#,
            r#"{"jsonrpc":"2.0","id":4,"result":{"stopReason":"end_turn"}}"#,
        ]);
        let mut writer = Vec::new();

        pump_until_response(&mut reader, &mut writer, 4, Duration::from_secs(5))
            .await
            .unwrap();

        let sent = String::from_utf8(writer).unwrap();
        assert!(sent.contains("\"id\":9"), "{sent}");
        assert!(sent.contains("-32601"), "{sent}");
    }

    #[tokio::test]
    async fn pump_maps_error_response() {
        let mut reader = frames(&[
            r#"{"jsonrpc":"2.0","id":5,"error":{"code":-32000,"message":"auth required"}}"#,
        ]);
        let mut writer = Vec::new();

        let err = pump_until_response(&mut reader, &mut writer, 5, Duration::from_secs(5))
            .await
            .unwrap_err();
        match err {
            JobsmithError::KimiProtocol(msg) => {
                assert!(msg.contains("auth required"), "{msg}");
                assert!(msg.contains("-32000"), "{msg}");
            }
            other => panic!("expected KimiProtocol, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn pump_rejects_output_over_limit() {
        let big = "x".repeat(MAX_OUTPUT_BYTES + 1);
        let chunk = format!(
            r#"{{"jsonrpc":"2.0","method":"session/update","params":{{"sessionId":"s","update":{{"sessionUpdate":"agent_message_chunk","content":{{"type":"text","text":"{big}"}}}}}}}}"#
        );
        let mut reader = frames(&[&chunk]);
        let mut writer = Vec::new();

        let err = pump_until_response(&mut reader, &mut writer, 1, Duration::from_secs(5))
            .await
            .unwrap_err();
        match err {
            JobsmithError::KimiProtocol(msg) => assert!(msg.contains("10 MiB"), "{msg}"),
            other => panic!("expected KimiProtocol, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn pump_errors_on_stream_eof() {
        let mut reader = frames(&[]);
        let mut writer = Vec::new();

        let err = pump_until_response(&mut reader, &mut writer, 1, Duration::from_secs(5))
            .await
            .unwrap_err();
        assert!(
            matches!(err, JobsmithError::KimiProtocol(ref m) if m.contains("closed")),
            "{err:?}"
        );
    }

    #[tokio::test]
    async fn pump_times_out_on_silent_stream() {
        // duplex write-end stays open and silent: read_line pends forever
        let (silent, _keep_open) = tokio::io::duplex(64);
        let mut reader = tokio::io::BufReader::new(silent);
        let mut writer = Vec::new();

        let err = pump_until_response(&mut reader, &mut writer, 1, Duration::from_millis(100))
            .await
            .unwrap_err();
        assert!(
            matches!(err, JobsmithError::ProcessTimeout { .. }),
            "{err:?}"
        );
    }

    /// A spawn against a process that never speaks ACP must fail with
    /// `ProcessTimeout` in bounded time instead of hanging.
    #[cfg(unix)]
    #[tokio::test]
    async fn spawn_times_out_on_hung_binary() {
        use std::os::unix::fs::PermissionsExt;

        let script =
            std::env::temp_dir().join(format!("jobsmith-hung-acp-{}.sh", std::process::id()));
        std::fs::write(&script, "#!/bin/sh\nsleep 600\n").unwrap();
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).unwrap();

        let result = tokio::time::timeout(
            Duration::from_secs(5),
            AcpClient::spawn_binary_with_timeout(
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

    /// Live roundtrip against the installed `kimi` binary.
    ///
    /// Spends real kimi tokens, so it is ignored by default:
    /// `cargo test --lib acp_live -- --ignored`
    #[ignore = "spends kimi tokens; requires an authenticated kimi-code CLI"]
    #[tokio::test]
    async fn acp_live_roundtrip() {
        let mut client = AcpClient::spawn().await.expect("kimi acp must spawn");
        let output = client
            .prompt_and_collect("Ответь ровно одним словом: ок".to_string())
            .await
            .expect("prompt must complete");
        client.shutdown().await.expect("shutdown must succeed");
        assert!(
            output.to_lowercase().contains("ок"),
            "unexpected output: {output:?}"
        );
    }
}
