//! Local IPC contract (WebView <-> Rust host and HTTP serve carrier).
//!
//! @capability contract.ipc
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// One command request. `payload` is command-specific and schema-validated at the boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CommandRequest {
    /// Command name, e.g. `chat.send`.
    pub name: String,
    /// Command payload (object).
    pub payload: Value,
}

/// One command response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    /// Error helper.
    pub fn err(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

/// Streaming event envelope. `kind` must be a stable string (e.g. `agent`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize, schemars::JsonSchema)]
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
    /// Answer a pending user question.
    pub const QUESTION_ANSWER: &str = "question.answer";
    /// List workspace files (bounded, read-only).
    pub const WORKSPACE_FILES: &str = "workspace.files";
    /// Check personal update feed.
    pub const UPDATE_CHECK: &str = "update.check";
    /// Forward one app request envelope to the server (Bearer injected by host).
    pub const APP_REQUEST: &str = "app_request";
    /// Install a downloaded skill archive (validated/extracted by host).
    pub const SKILLS_INSTALL: &str = "skills.install";
    /// Save connector OAuth tokens in the OS keychain.
    pub const OAUTH_TOKEN_SAVE: &str = "oauth.save";
    /// Load connector OAuth tokens from the OS keychain.
    pub const OAUTH_TOKEN_LOAD: &str = "oauth.load";
    /// Clear connector OAuth tokens from the OS keychain.
    pub const OAUTH_TOKEN_CLEAR: &str = "oauth.clear";
    /// Remove an installed skill (all versions).
    pub const SKILLS_REMOVE: &str = "skills.remove";
    /// List installed skills (host-managed).
    pub const SKILLS_INSTALLED: &str = "skills.installed";
    /// Runtime metadata (edition/packages).
    pub const META_GET: &str = "meta.get";
    /// Inject enterprise gateway server URL/access token after login.
    pub const AUTH_CONFIGURE: &str = "auth.configure";
    /// Clear enterprise gateway credentials after logout (local session is UI-owned).
    pub const AUTH_LOGOUT: &str = "auth.logout";
}

/// Typed command payloads (single source for `schemars` JSON Schema).
///
/// `CommandRequest::payload` stays an opaque [`serde_json::Value`] on the wire;
/// these structs describe the documented shapes from `docs/26` §5 plus the
/// settings/approval/question commands, so the boundary can be schema-validated
/// (C10/T22) without a second hand-written definition.
pub mod payloads {
    use serde::{Deserialize, Serialize};

    /// Empty payload (`meta.get`, `settings.get`, `workspace.list`, ...).
    #[derive(
        Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
    )]
    pub struct EmptyPayload {}

    /// `workspace.create` payload.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct WorkspaceCreatePayload {
        /// Local directory path (host canonicalizes it).
        pub path: String,
        /// Optional display title; empty means host default.
        #[serde(default)]
        pub title: String,
    }

    /// Commands that address one workspace (`workspace.set_default`, `session.list`).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct WorkspaceIdPayload {
        /// Target workspace id.
        pub workspace_id: String,
    }

    /// `workspace.rename` payload.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct WorkspaceRenamePayload {
        /// Target workspace id.
        pub workspace_id: String,
        /// New display title (must be non-empty after trim).
        pub title: String,
    }

    /// `workspace.remove` payload (destructive; `confirm` must be `true`).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct WorkspaceRemovePayload {
        /// Workspace to remove.
        pub workspace_id: String,
        /// Optional target workspace for conversations in the removed one.
        #[serde(default)]
        pub reassign_to: Option<String>,
        /// Explicit user confirmation.
        pub confirm: bool,
    }

    /// `session.create` payload.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct SessionCreatePayload {
        /// Owning workspace id (mandatory; sessions never auto-create).
        pub workspace_id: String,
        /// Optional conversation title.
        #[serde(default)]
        pub title: String,
    }

    /// `session.messages` payload.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct SessionMessagesPayload {
        /// Owning workspace id (ownership is verified).
        pub workspace_id: String,
        /// Conversation id.
        pub conversation_id: String,
    }

    /// `chat.send` payload (no auto-create; both ids are mandatory).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct ChatSendPayload {
        /// Owning workspace id.
        pub workspace_id: String,
        /// Existing conversation id.
        pub conversation_id: String,
        /// User message text.
        pub text: String,
    }

    /// `chat.cancel` payload (per-stream cancellation).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct ChatCancelPayload {
        /// Owning workspace id.
        pub workspace_id: String,
        /// Active stream id to cancel.
        pub stream_id: String,
    }

    /// `settings.set` payload.
    ///
    /// `AppSettings` carries `f64` completion parameters, so this DTO is
    /// `PartialEq` but not `Eq` (unchanged wire shape).
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct SettingsSetPayload {
        /// Full settings document (same schema as `zk-contract-config::AppSettings`).
        pub settings: zk_contract_config::AppSettings,
    }

    /// `approval.decide` payload.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct ApprovalDecidePayload {
        /// Pending approval id.
        pub approval_id: String,
        /// User decision.
        pub decision: zk_contract_agent::ApprovalDecision,
    }

    /// `question.answer` payload.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct QuestionAnswerPayload {
        /// Pending question batch id.
        pub question_id: String,
        /// Answers keyed by question id (opaque JSON).
        pub answers: serde_json::Value,
    }
}

/// Typed command results (single source for `schemars` JSON Schema).
///
/// Command responses stay `{ ok, data, error }` and `data` remains an opaque
/// [`serde_json::Value`] on the wire; these structs describe the documented
/// `data` shapes so the boundary can be schema-validated (C10/T22).
pub mod results {
    use serde::{Deserialize, Serialize};

    /// `meta.get` result.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct MetaInfo {
        /// Edition (`personal` | `enterprise`) baked in by the composed build.
        pub edition: String,
        /// Composed app version.
        pub version: String,
        /// Composed package ids.
        pub packages: Vec<String>,
        /// Boot-time agent config error when the runtime fell back to defaults.
        ///
        /// The host serializes `null` when there is no error; absent is accepted too.
        #[serde(default)]
        pub agent_config_error: Option<String>,
    }

    /// `chat.send` result (one accepted turn).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct ChatSendResult {
        /// Stream id used to correlate `UiEvent`s for this turn.
        pub stream_id: String,
        /// Conversation that accepted the turn.
        pub conversation_id: String,
        /// Owning workspace id (ownership is verified before accepting).
        pub workspace_id: String,
    }
}
