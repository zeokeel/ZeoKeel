//! Local agent loop contract (minimal M0 subset).
//!
//! @capability contract.agent
#![forbid(unsafe_code)]

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use zk_contract_tool::TurnContext;
use thiserror::Error;

/// One executed tool call, recorded for UI and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallRecord {
    /// Tool name, e.g. `fs.read`.
    pub name: String,
    /// Raw arguments passed to the tool.
    pub args: String,
    /// Tool output (or error text).
    pub result: String,
}

/// Result of one agent turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentReply {
    /// Final assistant text.
    pub text: String,
    /// Tool calls executed during the turn (empty when none).
    pub tool_calls: Vec<ToolCallRecord>,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    ApprovalRequested { approval_id: String, tool: String, args: String, reason: String },
    /// An approval request was resolved (UI may render the outcome).
    ApprovalResolved { approval_id: String, decision: ApprovalDecision },
    /// The agent asks the user questions and pauses until answered.
    QuestionAsked { question_id: String, questions: serde_json::Value },
    /// Questions were answered (or timed out with `null`).
    QuestionAnswered { question_id: String, answers: serde_json::Value },
    /// Turn finished successfully.
    TurnCompleted { reply: AgentReply },
    /// Turn was cancelled by the user.
    TurnCancelled,
    /// Turn failed.
    Error { message: String },
}

/// One approval request raised by a tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    /// Allow exactly this call.
    AllowOnce,
    /// Reject this call.
    Reject,
    /// Cancel the turn.
    Cancel,
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
        sink.emit(AgentEvent::TurnStarted { turn_id: turn_id.to_owned() });
        match self.run(input) {
            Ok(reply) => {
                sink.emit(AgentEvent::TextDelta { delta: reply.text.clone() });
                sink.emit(AgentEvent::TurnCompleted { reply: reply.clone() });
                Ok(reply)
            }
            Err(err) => {
                sink.emit(AgentEvent::Error { message: err.to_string() });
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

    /// Swap the underlying model/provider at runtime (settings hot-reload).
    fn configure_model(&self, _base_url: &str, _api_key: &str, _model: &str, _provider: &str) -> Result<(), String> {
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
        sink.emit(AgentEvent::TurnStarted { turn_id: turn_id.to_owned() });
        match self.run_in(context, input) {
            Ok(reply) => {
                sink.emit(AgentEvent::TextDelta { delta: reply.text.clone() });
                sink.emit(AgentEvent::TurnCompleted { reply: reply.clone() });
                Ok(reply)
            }
            Err(err) => {
                sink.emit(AgentEvent::Error { message: err.to_string() });
                Err(err)
            }
        }
    }
}
