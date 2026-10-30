//! Local IPC contract (WebView <-> Rust host and HTTP serve carrier).
//!
//! @capability contract.ipc
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// One command request. `payload` is command-specific and schema-validated at the boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandRequest {
    /// Command name, e.g. `chat.send`.
    pub name: String,
    /// Command payload (object).
    pub payload: Value,
}

/// One command response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandResponse {
    /// True on success.
    pub ok: bool,
    /// Success payload (if any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    /// Error message on failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl CommandResponse {
    /// Success helper.
    pub fn ok(data: Value) -> Self {
        Self { ok: true, data: Some(data), error: None }
    }

    /// Error helper.
    pub fn err(message: impl Into<String>) -> Self {
        Self { ok: false, data: None, error: Some(message.into()) }
    }
}

/// Streaming event envelope. `kind` must be a stable string (e.g. `agent`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiEvent {
    /// Stream id (one per turn/window).
    pub stream_id: String,
    /// Monotonic per stream.
    pub seq: u64,
    /// Event kind, e.g. `agent`.
    pub kind: String,
    /// Event payload.
    pub payload: Value,
}

/// Closed IPC error set.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum IpcError {
    /// Command name unknown.
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    /// Invalid payload.
    #[error("invalid payload: {0}")]
    InvalidPayload(String),
    /// Command failed.
    #[error("command failed: {0}")]
    Failed(String),
}

/// Stable command names (single source for host + UI + tests).
pub mod commands {
    /// Send a chat message and start a turn.
    pub const CHAT_SEND: &str = "chat.send";
    /// Cancel the active turn.
    pub const CHAT_CANCEL: &str = "chat.cancel";
    /// List workspaces.
    pub const WORKSPACE_LIST: &str = "workspace.list";
    /// Register/create a workspace.
    pub const WORKSPACE_CREATE: &str = "workspace.create";
    /// Rename a workspace.
    pub const WORKSPACE_RENAME: &str = "workspace.rename";
    /// Set the default workspace.
    pub const WORKSPACE_SET_DEFAULT: &str = "workspace.set_default";
    /// Remove a workspace (reassign conversations first).
    pub const WORKSPACE_REMOVE: &str = "workspace.remove";
    /// List conversations.
    pub const SESSION_LIST: &str = "session.list";
    /// Load messages for a conversation.
    pub const SESSION_MESSAGES: &str = "session.messages";
    /// Create a conversation.
    pub const SESSION_CREATE: &str = "session.create";
    /// Read settings.
    pub const SETTINGS_GET: &str = "settings.get";
    /// Save settings.
    pub const SETTINGS_SET: &str = "settings.set";
    /// Decide an approval request.
    pub const APPROVAL_DECIDE: &str = "approval.decide";
    /// List workspace files (bounded, read-only).
    pub const WORKSPACE_FILES: &str = "workspace.files";
    /// Check personal update feed.
    pub const UPDATE_CHECK: &str = "update.check";
    /// Runtime metadata (edition/packages).
    pub const META_GET: &str = "meta.get";
    /// Inject enterprise gateway server URL/access token after login.
    pub const AUTH_CONFIGURE: &str = "auth.configure";
}
