//! Observable agent lifecycle hooks (turn / tool / approval / usage).
//!
//! `cap-hooks` reports the capability as available because these hooks are
//! implemented by the shipped runtime; hosts install a [`HookSink`] (desktop
//! event bridge, JSONL recorder, tests) or set `HooksConfig::log` to append the
//! same events as JSONL. Events are additive: they never change the agent turn.
//!
//! # Log format and secrets
//!
//! One JSON object per line: `hook` (`turn_started`, `turn_completed`,
//! `tool_started`, `tool_finished`, `approval_requested`, `approval_resolved`,
//! `memory_unavailable`, `turn_failed`), `turn_id`, and per-hook fields:
//! `tool`, `result_chars`, `is_error`, `approval_id`, `allowed`,
//! `usage{input_tokens,output_tokens,total_tokens,cached_input_tokens,
//! cache_creation_input_tokens,reasoning_tokens,cost_usd}`, `cancelled`,
//! `text_chars`, `reason`, `message`. Tool arguments and prompts are **never**
//! logged: `tool_started.args` is emitted as `"[redacted]"`, and the file is
//! created `0600` inside a `0700` directory (Unix). Full arguments stay in the
//! in-process [`HookSink`] only.

use serde::{Deserialize, Serialize};

/// Token usage reported by the provider for one turn (rig `Usage`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UsageSnapshot {
    /// Every input token (cached + uncached).
    pub input_tokens: Option<u64>,
    /// Every output token, reasoning included.
    pub output_tokens: Option<u64>,
    /// `input_tokens + output_tokens`, when both were reported.
    pub total_tokens: Option<u64>,
    /// The part of `input_tokens` read from a provider-managed cache.
    pub cached_input_tokens: Option<u64>,
    /// The part of `input_tokens` written to a provider-managed cache.
    pub cache_creation_input_tokens: Option<u64>,
    /// The part of `output_tokens` spent on internal reasoning.
    pub reasoning_tokens: Option<u64>,
    /// Provider-reported cost in USD when known.
    pub cost_usd: Option<f64>,
}

impl From<rig_core::completion::Usage> for UsageSnapshot {
    fn from(usage: rig_core::completion::Usage) -> Self {
        Self {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            total_tokens: usage.total_tokens,
            cached_input_tokens: usage.cached_input_tokens,
            cache_creation_input_tokens: usage.cache_creation_input_tokens,
            reasoning_tokens: usage.reasoning_tokens,
            cost_usd: usage.cost.map(|cost| cost.total),
        }
    }
}

/// One observable agent lifecycle event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "hook", rename_all = "snake_case")]
pub enum HookEvent {
    /// A turn was accepted.
    TurnStarted {
        /// Turn id from the caller.
        turn_id: String,
        /// Conversation id (workspace-scoped).
        conversation_id: String,
        /// Workspace id.
        workspace_id: String,
    },
    /// A turn finished successfully or was cancelled.
    TurnCompleted {
        /// Turn id from the caller.
        turn_id: String,
        /// Provider usage for the turn, when reported.
        usage: Option<UsageSnapshot>,
        /// Whether the caller cancelled the turn.
        cancelled: bool,
        /// Length of the final assistant text in chars.
        text_chars: usize,
    },
    /// A tool call is about to run (after approval, when required).
    ToolStarted {
        /// Turn id from the caller.
        turn_id: String,
        /// Tool name.
        tool: String,
        /// Raw arguments JSON.
        args: String,
    },
    /// A tool call finished (success or error text).
    ToolFinished {
        /// Turn id from the caller.
        turn_id: String,
        /// Tool name.
        tool: String,
        /// Whether the tool reported an error.
        is_error: bool,
        /// Result length in chars.
        result_chars: usize,
    },
    /// An approval is required before a mutating tool runs.
    ApprovalRequested {
        /// Turn id from the caller.
        turn_id: String,
        /// Approval id.
        approval_id: String,
        /// Tool name.
        tool: String,
    },
    /// The approval gate decided.
    ApprovalResolved {
        /// Turn id from the caller.
        turn_id: String,
        /// Approval id.
        approval_id: String,
        /// Whether the call was allowed.
        allowed: bool,
    },
    /// Conversation memory was requested but no persistence directory is
    /// configured (so the turn continued without history replay).
    MemoryUnavailable {
        /// Turn id from the caller.
        turn_id: String,
        /// Why memory could not be enabled.
        reason: String,
    },
    /// The turn failed.
    TurnFailed {
        /// Turn id from the caller.
        turn_id: String,
        /// Error message surfaced to the host.
        message: String,
    },
}

/// Host-side hook consumer (desktop bridge, JSONL recorder, tests).
pub trait HookSink: Send + Sync {
    /// Receive one lifecycle event. Implementations must not block the turn.
    fn on_event(&self, event: &HookEvent);
}

pub(crate) fn emit_hook(
    config: &crate::HooksConfig,
    sink: Option<&std::sync::Arc<dyn HookSink>>,
    event: &HookEvent,
) {
    if !config.enabled {
        return;
    }
    if let Some(sink) = sink {
        // In-process consumers may need the full event (e.g. an approval UI
        // already renders the arguments). External sinks below never see them.
        sink.on_event(event);
    }
    let redacted = match serde_json::to_value(event) {
        Ok(value) => redact_event_value(value),
        Err(error) => {
            eprintln!("zk-agent-rig: hook serialize failed: {error}");
            return;
        }
    };
    if let Some(contract_sink) = zk_contract_agent::agent_hook_sink() {
        contract_sink.on_agent_hook(&redacted);
    }
    if config.log.trim().is_empty() {
        return;
    }
    let line = match serde_json::to_string(&redacted) {
        Ok(line) => line,
        Err(error) => {
            eprintln!("zk-agent-rig: hook serialize failed: {error}");
            return;
        }
    };
    match open_hook_log(std::path::Path::new(config.log.trim())) {
        Ok(mut file) => {
            use std::io::Write;
            if let Err(error) = writeln!(file, "{line}") {
                eprintln!("zk-agent-rig: hook log write failed: {error}");
            }
        }
        Err(error) => eprintln!("zk-agent-rig: hook log open failed: {error}"),
    }
}

/// Drop or summarize sensitive payloads before an event leaves the process
/// (`tool_started.args` may contain commands/keys/paths).
fn redact_event_value(mut value: serde_json::Value) -> serde_json::Value {
    if value.get("hook").and_then(serde_json::Value::as_str) == Some("tool_started") {
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "args".to_owned(),
                serde_json::Value::String("[redacted]".to_owned()),
            );
        }
    }
    value
}

/// Open (append) the JSONL hook log as `0600`, creating directories `0700`.
fn open_hook_log(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            create_private_dir(parent)?;
        }
    }
    let mut options = std::fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = file.metadata()?.permissions();
        permissions.set_mode(0o600);
        file.set_permissions(permissions)?;
    }
    Ok(file)
}

fn create_private_dir(dir: &std::path::Path) -> std::io::Result<()> {
    if dir.is_dir() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder.create(dir)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(dir)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only

    use super::*;

    #[test]
    fn tool_started_args_are_redacted_and_log_file_is_private() {
        let dir = std::env::temp_dir().join(format!(
            "zk-hooks-test-{}-{}",
            std::process::id(),
            crate::APPROVAL_SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let log = dir.join("private").join("hooks.jsonl");
        let config = crate::HooksConfig {
            enabled: true,
            log: log.to_string_lossy().to_string(),
        };
        emit_hook(
            &config,
            None,
            &HookEvent::ToolStarted {
                turn_id: "turn".to_owned(),
                tool: "bash".to_owned(),
                args: "{\"command\":\"echo SECRET-TOKEN\"}".to_owned(),
            },
        );
        let text = std::fs::read_to_string(&log).unwrap();
        assert!(text.contains("\"hook\":\"tool_started\""), "{text}");
        assert!(text.contains("\"args\":\"[redacted]\""), "{text}");
        assert!(!text.contains("SECRET-TOKEN"), "raw args leaked: {text}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&log).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "hook log must be 0600");
            let dir_mode = std::fs::metadata(dir.join("private"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(dir_mode, 0o700, "hook log directory must be 0700");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn redaction_only_touches_tool_started() {
        let completed = redact_event_value(serde_json::json!({
            "hook": "turn_completed",
            "turn_id": "t",
            "usage": {"input_tokens": 1}
        }));
        assert_eq!(completed["usage"]["input_tokens"], 1);
        let started = redact_event_value(serde_json::json!({
            "hook": "tool_started",
            "args": "SECRET"
        }));
        assert_eq!(started["args"], "[redacted]");
    }
}
