//! Local agent loop contract (minimal M0 subset).
//!
//! @capability contract.agent
#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use zk_contract_config::AgentSettings;
use zk_contract_tool::TurnContext;

/// One executed tool call, recorded for UI and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ToolCallRecord {
    /// Tool name, e.g. `fs.read`.
    pub name: String,
    /// Raw arguments passed to the tool.
    pub args: String,
    /// Tool output (or error text).
    pub result: String,
}

/// Token usage reported by the provider for one agent turn.
///
/// The counters are required (a `usage` object is only present when the
/// provider reported usage; unreported counters are normalized to `0` by the
/// runtime), matching `specs/changes/2026-10-31-config-and-media/spec.md`.
/// `cost_micros` stays optional because most providers do not report cost.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
pub struct AgentUsage {
    /// Every input token (cached + uncached).
    pub input_tokens: u64,
    /// Every output token, reasoning included.
    pub output_tokens: u64,
    /// `input_tokens + output_tokens`.
    pub total_tokens: u64,
    /// The part of `input_tokens` read from a provider-managed cache.
    pub cached_tokens: u64,
    /// The part of `output_tokens` spent on internal reasoning.
    pub reasoning_tokens: u64,
    /// Provider-reported cost in micro-USD (1e-6 USD), when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_micros: Option<u64>,
}

/// Result of one agent turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AgentReply {
    /// Final assistant text.
    pub text: String,
    /// Tool calls executed during the turn (empty when none).
    pub tool_calls: Vec<ToolCallRecord>,
    /// Provider token usage for the turn, when reported.
    #[serde(default)]
    pub usage: Option<AgentUsage>,
}

/// Closed error set for the agent loop.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AgentError {
    /// The provider failed.
    #[error("agent llm error: {0}")]
    Llm(String),
    /// A tool failed.
    #[error("agent tool error: {0}")]
    Tool(String),
    /// Input could not be interpreted.
    #[error("agent invalid input: {0}")]
    Invalid(String),
}

/// Streaming events emitted during one agent turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// Turn accepted and started.
    TurnStarted { turn_id: String },
    /// Incremental assistant text.
    TextDelta { delta: String },
    /// A tool call is about to run.
    ToolCallStarted { name: String, args: String },
    /// A tool call finished (success or error text in `result`).
    ToolCallFinished { name: String, result: String },
    /// The turn needs a user approval decision.
    ApprovalRequested {
        approval_id: String,
        tool: String,
        args: String,
        reason: String,
    },
    /// An approval request was resolved (UI may render the outcome).
    ApprovalResolved {
        approval_id: String,
        decision: ApprovalDecision,
    },
    /// The agent asks the user questions and pauses until answered.
    QuestionAsked {
        question_id: String,
        /// Wire stays an opaque JSON value (backward compatible); the generated
        /// schema is the typed `Vec<QuestionDef>` shape for C10/T22 validation.
        #[schemars(with = "Vec<QuestionDef>")]
        questions: serde_json::Value,
    },
    /// Questions were answered (or timed out with `null`).
    QuestionAnswered {
        question_id: String,
        answers: serde_json::Value,
    },
    /// Turn finished successfully.
    TurnCompleted { reply: AgentReply },
    /// Turn was cancelled by the user.
    TurnCancelled,
    /// Turn failed.
    Error { message: String },
}

/// One approval request raised by a tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ApprovalRequest {
    /// Stable id for this decision.
    pub approval_id: String,
    /// Tool name.
    pub tool: String,
    /// JSON arguments.
    pub args: String,
    /// Why the tool needs approval.
    pub reason: String,
}

/// Result of an approval request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    /// Allow exactly this call.
    AllowOnce,
    /// Reject this call.
    Reject,
    /// Cancel the turn.
    Cancel,
}

/// One selectable option of a user question (`ask_user_question` tool input).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct QuestionOption {
    /// Option label (the selected answer value).
    pub label: String,
    /// Optional secondary description shown under the label.
    #[serde(default)]
    pub description: String,
}

/// One question the agent asks before continuing (`ask_user_question` tool input).
///
/// Mirrors the tool's argument schema: `id`/`question` are required; `header`,
/// `options`, `multi_select` are optional and default to empty/false.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct QuestionDef {
    /// Stable question id (key in the answers map).
    pub id: String,
    /// Question text.
    pub question: String,
    /// Optional short header/category.
    #[serde(default)]
    pub header: String,
    /// Selectable options (empty for free-text questions).
    #[serde(default)]
    pub options: Vec<QuestionOption>,
    /// Whether multiple options may be selected.
    #[serde(default)]
    pub multi_select: bool,
}

/// Typed payload of [`AgentEvent::QuestionAsked`].
///
/// The event keeps `questions: serde_json::Value` on the wire for backward
/// compatibility; this DTO is the canonical typed shape for consumers that
/// validate or map the event (C10/T22).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct QuestionAskedPayload {
    /// Pending question batch id (also the `question.answer` correlation id).
    pub question_id: String,
    /// Questions in this batch (non-empty in practice).
    pub questions: Vec<QuestionDef>,
}

/// Sink for streaming events (UI/bridge implements it).
pub trait EventSink: Send + Sync {
    /// Emit one event; implementations must not block.
    fn emit(&self, event: AgentEvent);
}

/// A replaceable local agent loop (state machine grows in M1).
pub trait Agent: Send + Sync {
    /// Run one turn for the given user input.
    fn run(&self, input: &str) -> Result<AgentReply, AgentError>;

    /// Run one turn with an explicit workspace context.
    fn run_in(&self, _context: &TurnContext, input: &str) -> Result<AgentReply, AgentError> {
        self.run(input)
    }

    /// Run one turn and stream events. Default implementation emits start/complete only.
    fn run_stream(
        &self,
        turn_id: &str,
        input: &str,
        sink: &dyn EventSink,
    ) -> Result<AgentReply, AgentError> {
        sink.emit(AgentEvent::TurnStarted {
            turn_id: turn_id.to_owned(),
        });
        match self.run(input) {
            Ok(reply) => {
                sink.emit(AgentEvent::TextDelta {
                    delta: reply.text.clone(),
                });
                sink.emit(AgentEvent::TurnCompleted {
                    reply: reply.clone(),
                });
                Ok(reply)
            }
            Err(err) => {
                sink.emit(AgentEvent::Error {
                    message: err.to_string(),
                });
                Err(err)
            }
        }
    }

    /// Run with a shared sink so tools/approval gates can emit while executing.
    fn run_stream_in_shared(
        &self,
        context: &TurnContext,
        turn_id: &str,
        input: &str,
        sink: Arc<dyn EventSink>,
    ) -> Result<AgentReply, AgentError> {
        self.run_stream_in(context, turn_id, input, sink.as_ref())
    }

    /// Request cancellation of the currently running turn (best effort).
    fn cancel(&self) {}

    /// Apply runtime-configurable agent settings (ADR-064 / T22).
    ///
    /// The default implementation accepts but ignores the settings so existing
    /// implementations keep compiling; implementations that support them (e.g.
    /// `zk-agent-rig`) validate and apply them on the next turn.
    fn configure_agent(&self, _settings: &AgentSettings) -> Result<(), String> {
        Ok(())
    }

    /// Swap the underlying model/provider at runtime (settings hot-reload).
    fn configure_model(
        &self,
        _base_url: &str,
        _api_key: &str,
        _model: &str,
        _provider: &str,
    ) -> Result<(), String> {
        Err("agent does not support runtime model configuration".to_owned())
    }

    /// Run one turn with a workspace context and stream events.
    fn run_stream_in(
        &self,
        context: &TurnContext,
        turn_id: &str,
        input: &str,
        sink: &dyn EventSink,
    ) -> Result<AgentReply, AgentError> {
        sink.emit(AgentEvent::TurnStarted {
            turn_id: turn_id.to_owned(),
        });
        match self.run_in(context, input) {
            Ok(reply) => {
                sink.emit(AgentEvent::TextDelta {
                    delta: reply.text.clone(),
                });
                sink.emit(AgentEvent::TurnCompleted {
                    reply: reply.clone(),
                });
                Ok(reply)
            }
            Err(err) => {
                sink.emit(AgentEvent::Error {
                    message: err.to_string(),
                });
                Err(err)
            }
        }
    }
}

/// Process-wide runtime settings registry seam.
///
/// The desktop owns the settings store and the agent implementation
/// (`zk-agent-rig`) owns the turn loop; neither depends on the other, so the
/// resolved `AppSettings.agent` is published here (contract seam, C4) and the
/// runtime reads it on every turn. `None` means "never configured" so the
/// runtime may fall back to its environment defaults.
static AGENT_SETTINGS: OnceLock<Mutex<Option<AgentSettings>>> = OnceLock::new();
static AGENT_HOOK_SINK: OnceLock<Mutex<Option<Arc<dyn AgentHookSink>>>> = OnceLock::new();

fn agent_settings_slot() -> &'static Mutex<Option<AgentSettings>> {
    AGENT_SETTINGS.get_or_init(|| Mutex::new(None))
}

fn agent_hook_slot() -> &'static Mutex<Option<Arc<dyn AgentHookSink>>> {
    AGENT_HOOK_SINK.get_or_init(|| Mutex::new(None))
}

/// Publish the resolved agent settings for the local agent runtime.
///
/// # Errors
///
/// [`zk_contract_config::ConfigError::Invalid`] when the settings fail
/// validation; the previous value is kept.
pub fn set_default_agent_settings(settings: &AgentSettings) -> Result<(), String> {
    settings.validate().map_err(|error| error.to_string())?;
    let mut slot = agent_settings_slot()
        .lock()
        .map_err(|_| "agent settings registry poisoned".to_owned())?;
    *slot = Some(settings.clone());
    Ok(())
}

/// The published agent settings, or `None` when never configured.
#[must_use]
pub fn default_agent_settings() -> Option<AgentSettings> {
    agent_settings_slot()
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
}

/// Clear the published agent settings (save-non-default then reset-to-default).
///
/// After clearing, runtimes fall back to their env/default merge instead of a
/// stale default-valued registry entry.
///
/// # Errors
///
/// [`String`] when the registry lock is poisoned.
pub fn clear_default_agent_settings() -> Result<(), String> {
    let mut slot = agent_settings_slot()
        .lock()
        .map_err(|_| "agent settings registry poisoned".to_owned())?;
    *slot = None;
    Ok(())
}

/// One serialized lifecycle hook event (the JSON form of the runtime's
/// `HookEvent`), so hosts observe hooks without depending on agent-rig types.
pub trait AgentHookSink: Send + Sync {
    /// Receive one serialized hook event; implementations must not block.
    fn on_agent_hook(&self, event: &serde_json::Value);
}

/// Install the process-wide hook sink (desktop event bridge / JSONL logger).
pub fn set_agent_hook_sink(sink: Option<Arc<dyn AgentHookSink>>) {
    if let Ok(mut slot) = agent_hook_slot().lock() {
        *slot = sink;
    }
}

/// The process-wide hook sink, when installed.
#[must_use]
pub fn agent_hook_sink() -> Option<Arc<dyn AgentHookSink>> {
    agent_hook_slot().lock().ok().and_then(|slot| slot.clone())
}
