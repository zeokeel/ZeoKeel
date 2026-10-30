//! Observable agent lifecycle hooks (turn / tool / approval / usage).
//!
//! `cap-hooks` reports the capability as available because these hooks are
//! implemented by the shipped runtime; hosts install a [`HookSink`] (desktop
//! event bridge, JSONL recorder, tests) or set `HooksConfig::log` to append the
//! same events as JSONL. Events are additive: they never change the agent turn.

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
        sink.on_event(event);
    }
    if config.log.trim().is_empty() {
        return;
    }
    let line = match serde_json::to_string(event) {
        Ok(line) => line,
        Err(error) => {
            eprintln!("zk-agent-rig: hook serialize failed: {error}");
            return;
        }
    };
    let path = std::path::Path::new(config.log.trim());
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        Ok(mut file) => {
            use std::io::Write;
            if let Err(error) = writeln!(file, "{line}") {
                eprintln!("zk-agent-rig: hook log write failed: {error}");
            }
        }
        Err(error) => eprintln!("zk-agent-rig: hook log open failed: {error}"),
    }
}
