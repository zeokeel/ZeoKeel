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
    /// List read-only git working-tree changes for one workspace.
    pub const WORKSPACE_CHANGES: &str = "workspace.changes";
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
    /// Start the MCP connector OAuth 2.0 public-client + PKCE flow (docs/07 §6).
    ///
    /// Host-side: binds an ephemeral 127.0.0.1 loopback redirect (RFC 8252) and opens
    /// the system browser; returns immediately (`started`) — poll [`OAUTH_STATUS`].
    pub const OAUTH_AUTHORIZE: &str = "oauth.authorize";
    /// Poll a started connector OAuth flow; never returns tokens (`idle|pending|authorized|failed`).
    pub const OAUTH_STATUS: &str = "oauth.status";
    /// Refresh a stored connector OAuth token in the background (poll [`OAUTH_STATUS`]).
    pub const OAUTH_REFRESH: &str = "oauth.refresh";
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
    /// Open an http(s) URL in the platform browser (fixed argv; fire-and-forget).
    pub const SYSTEM_OPEN_URL: &str = "system.open_url";
    /// Install/refresh the enterprise connector catalog (sanitized, non-secret).
    pub const CONNECTORS_CONFIGURE: &str = "connectors.configure";
    /// Drop cached MCP sessions so the next call re-initializes from config.
    pub const CONNECTORS_RELOAD: &str = "connectors.reload";
    /// Clear the catalog-provided connector config (logout/disconnect).
    pub const CONNECTORS_CLEAR: &str = "connectors.clear";
    /// Start a no-cache MCP server probe (returns a `test_id`; poll `mcp.test.status`).
    pub const MCP_TEST: &str = "mcp.test";
    /// Poll one MCP probe (`pending|ok|error`).
    pub const MCP_TEST_STATUS: &str = "mcp.test.status";
    /// Persist one composer attachment inside the workspace
    /// (`2026-11-01-client-composer-surface`; base64 payload, host re-validates).
    pub const WORKSPACE_ATTACHMENT_SAVE: &str = "workspace.attachment_save";
    /// Read one workspace file for preview (text/image, honest degradation).
    pub const WORKSPACE_FILE_READ: &str = "workspace.file_read";
    /// User-driven persistent terminal (open/send/close/list/status).
    pub const TERMINAL: &str = "terminal";
    /// Rename/pin/archive one conversation.
    pub const SESSION_UPDATE: &str = "session.update";
    /// Fork one conversation inside its workspace (C18).
    pub const SESSION_FORK: &str = "session.fork";
    /// Full-text search inside the current workspace (snippet results).
    pub const SESSION_SEARCH: &str = "session.search";
    /// Read the current conversation goal (same `.zeokeel/goal.json` as `tool.goal`).
    pub const GOAL_GET: &str = "goal.get";
    /// Set/replace the current conversation goal (inline confirmation in the UI).
    pub const GOAL_SET: &str = "goal.set";
    /// Pause/resume/cancel the current conversation goal.
    pub const GOAL_UPDATE: &str = "goal.update";
    /// Read the conversation-scoped model/permission override.
    pub const SESSION_OVERRIDE_GET: &str = "session.override_get";
    /// Write the conversation-scoped model/permission override (validated by the host).
    pub const SESSION_OVERRIDE_SET: &str = "session.override_set";
    /// Export one conversation as Markdown into `<workspace>/.zeokeel/exports/`.
    pub const SESSION_EXPORT: &str = "session.export";
    /// Store local-only feedback JSON under `<workspace>/.zeokeel/feedback/`.
    pub const FEEDBACK_SUBMIT: &str = "feedback.submit";
    /// Compact older conversation history into one system marker (`/compact`).
    pub const SESSION_COMPACT: &str = "session.compact";
    /// Read the latest compaction marker of one conversation.
    pub const SESSION_COMPACTION_GET: &str = "session.compaction_get";
    /// Read the compacted replay view (marker + recent raw messages; read-only).
    pub const SESSION_COMPACTED_VIEW: &str = "session.compacted_view";
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
        /// Optimistic-concurrency fence: the `revision` the client last read.
        ///
        /// `None` (absent) keeps the legacy behaviour: no fence, always save
        /// (old clients / old documents stay compatible). A mismatch is rejected
        /// by the host without writing.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub expected_revision: Option<u64>,
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

    /// `mcp.test` payload: one draft server (not persisted; no global session).
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct McpTestPayload {
        /// Draft server config (validated by the host before probing).
        pub server: zk_contract_connector::McpServerConfig,
    }

    /// `connectors.configure` payload.
    ///
    /// `catalog` is the raw server response (`{items:[...]}`) or its JSON
    /// string form; the host validates it through the connector contract before
    /// touching the live MCP config (invalid payloads keep the old config).
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct ConnectorsConfigurePayload {
        /// Connector catalog (`{items:[...]}` object or JSON string).
        pub catalog: serde_json::Value,
    }

    /// `auth.configure` payload (additive `models` = enterprise gateway model cache).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct AuthConfigurePayload {
        /// Gateway server URL.
        #[serde(default)]
        pub server_url: String,
        /// Gateway access token.
        #[serde(default)]
        pub access_token: String,
        /// Gateway-provided model ids (host caches them; UI only displays them).
        #[serde(default)]
        pub models: Vec<String>,
    }

    /// `workspace.attachment_save` payload (composer attachment, additive).
    ///
    /// The host re-validates everything (workspace/conversation ownership, byte
    /// size after decode, extension/magic allow-list, filename sanitize) and
    /// returns the canonical metadata; the client hints are not trusted.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct AttachmentSavePayload {
        /// Owning workspace id.
        pub workspace_id: String,
        /// Conversation id; must belong to `workspace_id`.
        pub conversation_id: String,
        /// Client-declared basename (host sanitizes; path separators rejected).
        pub name: String,
        /// Client-declared MIME (host re-derives it from the extension/magic).
        pub mime: String,
        /// Base64 (standard alphabet, padded) of the raw file bytes.
        pub data_base64: String,
    }

    /// `workspace.files` payload (lazy tree; the no-argument form keeps the old
    /// depth≤2/≤300 behaviour, `2026-11-01-client-side-panels` R1).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct WorkspaceFilesPayload {
        /// Owning workspace id.
        pub workspace_id: String,
        /// Workspace-relative directory (`""` = workspace root).
        #[serde(default)]
        pub path: String,
        /// Depth relative to `path` (1..=2; absent keeps the legacy depth 2).
        #[serde(default)]
        pub depth: Option<u32>,
        /// Maximum entries (1..=500; absent keeps the legacy 300).
        #[serde(default)]
        pub limit: Option<u32>,
    }

    /// `workspace.file_read` payload (preview; read-only).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct WorkspaceFileReadPayload {
        /// Owning workspace id.
        pub workspace_id: String,
        /// Workspace-relative path (forward slashes).
        pub path: String,
        /// Optional caller byte budget (host clamps: text 256 KiB/image 8 MiB).
        #[serde(default)]
        pub max_bytes: Option<u64>,
    }

    /// One user-driven terminal action (`terminal` command).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    #[serde(rename_all = "lowercase")]
    pub enum TerminalAction {
        /// Open (or return) the persistent shell for a conversation.
        Open,
        /// Send one command line to the persistent shell.
        Send,
        /// Close the shell and drop it from the list.
        Close,
        /// List the conversation's live shells.
        List,
        /// Read one shell's accumulated output.
        Status,
    }

    /// `terminal` payload (`2026-11-01-client-side-panels` R3).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct TerminalPayload {
        /// Owning workspace id (the shell runs with this workspace as cwd).
        pub workspace_id: String,
        /// Conversation that owns the shell (isolation scope).
        pub conversation_id: String,
        /// Action to perform.
        pub action: TerminalAction,
        /// Target shell session id (`send`/`close`/`status`).
        #[serde(default)]
        pub session: Option<String>,
        /// Command line (`send`); passed to the persistent shell verbatim.
        #[serde(default)]
        pub command: Option<String>,
        /// Per-command timeout in ms (host clamps to ≤10_000).
        #[serde(default)]
        pub timeout_ms: Option<u64>,
    }

    /// `session.update` payload (rename/pin/archive; at least one field).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct SessionUpdatePayload {
        /// Owning workspace id.
        pub workspace_id: String,
        /// Conversation id.
        pub conversation_id: String,
        /// New title (trimmed, 1..=200 chars) when present.
        #[serde(default)]
        pub title: Option<String>,
        /// Pin/unpin when present.
        #[serde(default)]
        pub pinned: Option<bool>,
        /// Archive/unarchive when present.
        #[serde(default)]
        pub archived: Option<bool>,
    }

    /// `session.fork` payload (same-workspace copy, C18).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct SessionForkPayload {
        /// Owning workspace id (fork never crosses workspaces).
        pub workspace_id: String,
        /// Source conversation id.
        pub conversation_id: String,
        /// Optional new title; absent derives "<title> 副本".
        #[serde(default)]
        pub title: Option<String>,
    }

    /// `goal.get` payload.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct GoalGetPayload {
        /// Owning workspace id.
        pub workspace_id: String,
        /// Conversation id (goal is conversation-scoped).
        pub conversation_id: String,
    }

    /// One goal status action (`goal.update`).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    #[serde(rename_all = "lowercase")]
    pub enum GoalAction {
        /// Pause the goal (status `blocked`, reason `paused by user`).
        Pause,
        /// Resume a paused goal (status `active`).
        Resume,
        /// Cancel the goal (status `cancelled`, keeps the round log).
        Cancel,
    }

    /// `goal.set` payload (`objective` trimmed, 1..=4096 bytes).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct GoalSetPayload {
        /// Owning workspace id.
        pub workspace_id: String,
        /// Conversation id.
        pub conversation_id: String,
        /// Goal objective.
        pub objective: String,
        /// Optional round cap (host clamps 1..=100, default 10).
        #[serde(default)]
        pub max_rounds: Option<u32>,
    }

    /// `goal.update` payload.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct GoalUpdatePayload {
        /// Owning workspace id.
        pub workspace_id: String,
        /// Conversation id.
        pub conversation_id: String,
        /// Pause/resume/cancel.
        pub action: GoalAction,
    }

    /// `session.override_get` payload.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct ConversationOverrideGetPayload {
        /// Owning workspace id.
        pub workspace_id: String,
        /// Conversation id.
        pub conversation_id: String,
    }

    /// `session.override_set` payload (`None` clears that field).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct ConversationOverrideSetPayload {
        /// Owning workspace id.
        pub workspace_id: String,
        /// Conversation id.
        pub conversation_id: String,
        /// Session model override (`None` = clear; `Some("")` is rejected).
        #[serde(default)]
        pub model: Option<String>,
        /// Session reasoning effort (`minimal|low|medium|high|xhigh|max`).
        #[serde(default)]
        pub reasoning_effort: Option<String>,
        /// Session approval policy preset (`strict|auto_read_only|ask`).
        #[serde(default)]
        pub approval_policy: Option<String>,
    }

    /// `session.export` payload.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct SessionExportPayload {
        /// Owning workspace id.
        pub workspace_id: String,
        /// Conversation id.
        pub conversation_id: String,
        /// Export format (only `markdown` today).
        pub format: String,
    }

    /// `feedback.submit` payload (local-only; no network).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct FeedbackSubmitPayload {
        /// Owning workspace id.
        pub workspace_id: String,
        /// Conversation id (context only).
        pub conversation_id: String,
        /// Category: `bug` | `idea` | `other`.
        pub category: String,
        /// Optional description (≤4 KiB).
        #[serde(default)]
        pub description: Option<String>,
    }

    /// `session.compact` payload.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct SessionCompactPayload {
        /// Owning workspace id.
        pub workspace_id: String,
        /// Conversation id.
        pub conversation_id: String,
    }

    /// `session.compaction_get` / `session.compacted_view` payload.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct CompactionGetPayload {
        /// Owning workspace id.
        pub workspace_id: String,
        /// Conversation id.
        pub conversation_id: String,
    }

    /// `session.search` payload (current workspace only, C18).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct SessionSearchPayload {
        /// Owning workspace id.
        pub workspace_id: String,
        /// Query (trimmed; empty = no request, ≤200 chars enforced host-side).
        pub query: String,
        /// Result limit (1..=50; host default 20).
        #[serde(default)]
        pub limit: Option<u32>,
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

    /// One changed path from `workspace.changes`.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct WorkspaceChange {
        /// Path relative to the workspace root (forward slashes when git reports them).
        pub path: String,
        /// Coarse added/untracked/modified marker (`1` when this path is not a deletion).
        pub add: u32,
        /// Coarse deleted marker (`1` when git reports a deletion).
        pub del: u32,
    }

    /// Conversation goal view (mirrors the `tool.goal` driver JSON subset).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct GoalView {
        /// Objective text.
        pub objective: String,
        /// `active` | `achieved` | `cancelled` | `blocked`.
        pub status: String,
        /// Executed/current round.
        pub rounds: u32,
        /// Maximum rounds.
        pub max_rounds: u32,
        /// Optional reason (`paused by user`, `cancelled`, ...).
        #[serde(default)]
        pub reason: Option<String>,
        /// Last round result summary.
        #[serde(default)]
        pub last_result: Option<String>,
        /// Creation time (ms since epoch).
        #[serde(default)]
        pub created_ms: u64,
        /// Last update time (ms since epoch).
        #[serde(default)]
        pub updated_ms: u64,
    }

    /// `goal.get` result (`None` when the conversation has no goal).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct GoalResult {
        /// Current goal, if any.
        #[serde(default)]
        pub goal: Option<GoalView>,
    }

    /// Conversation-scoped override (model/effort/approval policy).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct ConversationOverride {
        /// Session model override.
        #[serde(default)]
        pub model: Option<String>,
        /// Session reasoning effort.
        #[serde(default)]
        pub reasoning_effort: Option<String>,
        /// Session approval policy preset.
        #[serde(default)]
        pub approval_policy: Option<String>,
        /// Whether any field is set for this conversation.
        pub is_session_override: bool,
    }

    /// `session.override_get` / `session.override_set` result.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct ConversationOverrideResult {
        /// Effective conversation override after the call.
        pub overrides: ConversationOverride,
    }

    /// `session.export` result.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct SessionExportResult {
        /// Absolute path of the generated Markdown file.
        pub path: String,
        /// File size in bytes.
        pub bytes: u64,
        /// MIME type (`text/markdown`).
        pub mime: String,
        /// More content existed than the 8 MiB export cap.
        pub truncated: bool,
    }

    /// `feedback.submit` result (local store only).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct FeedbackSubmitResult {
        /// Always `local` (no network submission).
        pub stored: String,
        /// Absolute path of the written JSON file.
        pub path: String,
    }

    /// Persisted conversation compaction marker (`role=system`, id `compaction:<uuid>`).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct CompactionMarker {
        /// Marker message id (`compaction:<uuid>`).
        pub marker_id: String,
        /// Extractive/model summary (≤2k tokens by contract).
        pub summary: String,
        /// Last raw message covered by the summary.
        pub through_message_id: String,
        /// Marker creation time (ms since epoch).
        pub created_ms: u64,
        /// Estimated/actual input tokens summarized.
        pub input_tokens: u64,
        /// Estimated/actual summary tokens.
        pub output_tokens: u64,
        /// Number of raw messages folded into the summary.
        pub compressed_messages: u32,
        /// Recent raw messages/turns kept verbatim.
        pub keep_recent: u32,
    }

    /// `session.compact` result (`noop` = boundary unchanged, nothing written).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct CompactResult {
        /// Current marker after the call (unchanged on noop/failure).
        #[serde(default)]
        pub marker: Option<CompactionMarker>,
        /// Nothing new to compact (idempotent repeat).
        pub noop: bool,
    }

    /// `session.compaction_get` result.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct CompactionGetResult {
        /// Latest marker, if any.
        #[serde(default)]
        pub marker: Option<CompactionMarker>,
    }

    /// `session.compacted_view` result (replay/divider only; raw store untouched).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct CompactedViewResult {
        /// Latest marker, if any.
        #[serde(default)]
        pub marker: Option<CompactionMarker>,
        /// Replay view: the marker message followed by recent raw messages.
        pub messages: Vec<zk_contract_session::Message>,
    }

    /// `terminal` result: live shells + one output snapshot.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct TerminalResult {
        /// Live shell session ids of the conversation (newest last).
        pub sessions: Vec<String>,
        /// New output since the previous read (UTF-8 lossy, ≤64 KiB).
        pub output: String,
        /// Output exceeded the per-call cap and was truncated.
        pub truncated: bool,
        /// Command exit code for `send` (when the shell reports one).
        #[serde(default)]
        pub exit_code: Option<i32>,
        /// Per-call timeout (ms) was hit; the shell stays usable.
        #[serde(default)]
        pub timed_out: bool,
    }

    /// `workspace.changes` result: bounded, read-only git working-tree summary.
    ///
    /// `warning` is present when git is unavailable / the workspace is not a
    /// repository / the output was truncated; `items` is empty in those cases
    /// where no reliable listing is available.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct WorkspaceChangesResult {
        /// Changed paths, sorted by path (bounded by the host).
        pub items: Vec<WorkspaceChange>,
        /// Human-readable non-fatal warning (omitted when the scan was clean).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub warning: Option<String>,
    }

    /// `settings.get` result: the stored settings plus their revision fence.
    ///
    /// `revision` is a monotonically increasing save counter kept next to the
    /// settings document (`settings.meta.json`); it is `0` for a fresh/invalid
    /// store. Passing it back as `expected_revision` on `settings.set` rejects
    /// concurrent overwrites.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct SettingsGetResult {
        /// Stored settings document (secrets masked by the host).
        pub settings: zk_contract_config::AppSettings,
        /// Current revision (increments on every successful save).
        pub revision: u64,
    }

    /// `settings.set` result: the persisted settings plus their new revision.
    ///
    /// Same shape as [`SettingsGetResult`] so the client does not need a second
    /// read after saving.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct SettingsSetResult {
        /// Persisted settings document (secrets masked by the host).
        pub settings: zk_contract_config::AppSettings,
        /// Revision after this save (`previous + 1`).
        pub revision: u64,
    }

    /// `connectors.configure` result: number of MCP servers installed.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct ConnectorsConfigureResult {
        /// Enabled servers installed from the catalog (0 when none are enabled).
        pub configured: u64,
    }

    /// `connectors.reload` result: whether cached MCP sessions were dropped.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct ConnectorsReloadResult {
        /// True on success (the command fails instead of returning false).
        pub reloaded: bool,
    }

    /// `connectors.clear` result: whether the catalog config was cleared.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct ConnectorsClearResult {
        /// True on success (the command fails instead of returning false).
        pub cleared: bool,
    }

    /// `mcp.test` result: opaque one-shot probe id (`pending` starts immediately).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct McpTestResult {
        /// Probe id to poll through `mcp.test.status`.
        pub test_id: String,
    }

    /// Closed probe state (C10: no bare `String` on the wire).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    #[serde(rename_all = "snake_case")]
    pub enum McpTestState {
        /// Probe still running.
        Pending,
        /// Probe finished successfully (`tools` populated).
        Ok,
        /// Probe failed (`error` carries a closed error code).
        Error,
    }

    /// `mcp.test.status` result: state + tools/error.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct McpTestStatusResult {
        /// Probe state.
        pub state: McpTestState,
        /// Tool names reported by the server (`ok` only; host caps at 200).
        #[serde(default)]
        pub tools: Vec<String>,
        /// Closed error code when `state == error` (e.g. `mcp/timeout`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub error: Option<String>,
    }

    /// `workspace.attachment_save` result (host-canonical metadata).
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
    pub struct AttachmentSaveResult {
        /// Workspace-relative path (forward slashes), e.g.
        /// `.zeokeel/attachments/<conversation>/<uuid>-<name>`.
        pub path: String,
        /// Sanitized display name (basename only).
        pub name: String,
        /// Decoded byte length actually written.
        pub bytes: u64,
        /// Canonical MIME derived by the host (extension + magic).
        pub mime: String,
    }
}
