//! Conversation/workspace persistence contract (M1 workspace-scoped subset).
//!
//! @capability contract.session
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One workspace: a local directory that owns conversations and agent cwd.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Workspace {
    /// Stable id (uuid string).
    pub id: String,
    /// Canonical local absolute path.
    pub path: String,
    /// Display title.
    pub title: String,
    /// Exactly one workspace is the default.
    pub is_default: bool,
    /// Creation time (ms since epoch).
    pub created_at_ms: u64,
    /// Last update time (ms since epoch).
    pub updated_at_ms: u64,
}

/// Result of removing a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RemoveWorkspaceResult {
    /// Always true when the call succeeds.
    pub removed: bool,
    /// Number of conversations moved to `reassign_to`.
    pub moved_sessions: u64,
}

/// One conversation (message thread), always owned by exactly one workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Conversation {
    /// Stable id (uuid/ulid string).
    pub id: String,
    /// Owning workspace id (immutable except explicit remove reassign).
    pub workspace_id: String,
    /// Display title.
    pub title: String,
    /// Creation time (ms since epoch).
    pub created_at_ms: u64,
    /// Last update time (ms since epoch).
    pub updated_at_ms: u64,
    /// Pinned to the top of its workspace list (Phase 2b, migration v6).
    #[serde(default)]
    pub pinned: bool,
    /// Archived: hidden from the default list, messages kept (Phase 2b, v6).
    #[serde(default)]
    pub archived: bool,
}

/// Kind of a `workspace.file_read` result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FileReadKind {
    /// UTF-8 text content (`encoding = utf8`).
    Text,
    /// Raster image content (`encoding = base64`).
    Image,
    /// Honest degradation: PDF/Office/HTML/binary/too large/symlink escape.
    Unsupported,
}

/// One `workspace.file_read` result (read-only, workspace relative).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceFileReadResult {
    /// Workspace-relative path (forward slashes).
    pub path: String,
    /// Content kind.
    pub kind: FileReadKind,
    /// Detected MIME (`text/*`/`image/*`; may be `application/octet-stream`).
    pub mime: String,
    /// Full file size in bytes.
    pub size: u64,
    /// Content was truncated to the host limit (text only).
    pub truncated: bool,
    /// `utf8` | `base64` | empty (unsupported).
    pub encoding: String,
    /// Content (empty when unsupported).
    pub content: String,
    /// User-readable degradation reason (empty when supported).
    #[serde(default)]
    pub reason: String,
}

/// One session full-text search hit (snippet, not full content).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SessionSearchHit {
    /// Conversation id.
    pub conversation_id: String,
    /// Conversation title.
    pub title: String,
    /// Matched message id.
    pub message_id: String,
    /// Message role (`user`/`assistant`/`tool`/`system`).
    pub role: String,
    /// Context window around the match (±60 chars).
    pub snippet: String,
    /// Conversation is archived (shown with a marker, still openable).
    pub archived: bool,
}

/// `session.search` result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SessionSearchResult {
    /// Hits in store order (pinned first, then newest).
    pub items: Vec<SessionSearchHit>,
    /// More hits existed than the effective limit.
    pub truncated: bool,
}

/// One message inside a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Message {
    /// Stable id.
    pub id: String,
    /// Parent conversation id.
    pub conversation_id: String,
    /// Role: `user` | `assistant` | `tool` | `system`.
    pub role: String,
    /// Text content (tool records are JSON strings in M1).
    pub content: String,
    /// Creation time (ms since epoch).
    pub created_at_ms: u64,
    /// Turn status: `done` | `cancelled` | `error` (M1 persistence for cancel markers).
    #[serde(default = "done_status")]
    pub status: String,
    /// Provider token usage for an assistant turn, when reported.
    ///
    /// Persisted so usage survives restarts (round-4); legacy rows parse as
    /// `None`.
    #[serde(default)]
    pub usage: Option<zk_contract_agent::AgentUsage>,
    /// Owning agent turn; tool rows and the closing assistant row share it.
    ///
    /// Legacy rows parse as an empty string and are grouped by the legacy
    /// fallback rule (next assistant in the conversation).
    #[serde(default)]
    pub turn_id: String,
}

fn done_status() -> String {
    "done".to_owned()
}

/// Kind of one workspace file-listing entry (`workspace.files`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    /// Directory (may be expanded).
    Dir,
    /// Regular file.
    File,
}

/// One bounded entry of a read-only workspace file listing (`workspace.files`).
///
/// Mirrors the host emission `{ "path", "depth", "kind" }` in
/// `apps/desktop/src/ipc.rs::list_workspace_files`; `badge` is optional UI
/// metadata for inspectors (absent on the current wire).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FileEntry {
    /// Path relative to the workspace root (forward slashes; depth-bounded).
    pub path: String,
    /// Depth relative to the workspace root (`0` = direct child).
    pub depth: u32,
    /// Entry kind.
    pub kind: FileKind,
    /// Optional inspector badge (empty/absent when the entry has none).
    #[serde(default)]
    pub badge: String,
}

/// `workspace.files` result: bounded read-only entries under one workspace root.
///
/// The host emits `{ "items": [...] }` (`list_workspace_files`); the field is
/// named `items` to stay wire-compatible with the current desktop/web parsers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceFilesResult {
    /// Entries sorted by path (bounded by the host).
    pub items: Vec<FileEntry>,
}

/// Closed error set for workspace/session storage.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SessionError {
    /// Conversation not found.
    #[error("conversation not found: {0}")]
    NotFound(String),
    /// Workspace not found.
    #[error("workspace not found: {0}")]
    WorkspaceNotFound(String),
    /// Conversation does not belong to the given workspace.
    #[error(
        "session/workspace-mismatch: conversation {conversation} is not in workspace {workspace}"
    )]
    WorkspaceMismatch {
        /// Conversation id.
        conversation: String,
        /// Expected workspace id.
        workspace: String,
    },
    /// Workspace path does not exist.
    #[error("workspace path not found: {0}")]
    WorkspacePathNotFound(String),
    /// Workspace path is not a directory.
    #[error("workspace not a directory: {0}")]
    WorkspaceNotADirectory(String),
    /// Workspace path is not writable.
    #[error("workspace not writable: {0}")]
    WorkspaceNotWritable(String),
    /// Workspace path is unavailable.
    #[error("workspace path unavailable: {0}")]
    WorkspacePathUnavailable(String),
    /// Reassign target is unavailable/invalid.
    #[error("workspace reassign target invalid: {0}")]
    WorkspaceReassignTargetInvalid(String),
    /// Default workspace cannot be removed directly.
    #[error("workspace is default: {0}")]
    DefaultWorkspace(String),
    /// Workspace path already registered.
    #[error("workspace duplicate path: {0}")]
    DuplicateWorkspacePath(String),
    /// Workspace has conversations and no reassign target.
    #[error("workspace in use: {0}")]
    WorkspaceInUse(String),
    /// Confirmation is required for a destructive workspace operation.
    #[error("confirmation required: {0}")]
    ConfirmRequired(String),
    /// Invalid workspace input/state.
    #[error("workspace invalid: {0}")]
    InvalidWorkspace(String),
    /// Storage failure.
    #[error("session storage error: {0}")]
    Storage(String),
    /// Invalid input.
    #[error("session invalid input: {0}")]
    Invalid(String),
}

impl SessionError {
    /// Stable IPC error code (closed, documented in `docs/26` §5).
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "session/not-found",
            Self::WorkspaceNotFound(_) => "workspace/not-found",
            Self::WorkspaceMismatch { .. } => "session/workspace-mismatch",
            Self::WorkspacePathNotFound(_) => "workspace/path-not-found",
            Self::WorkspaceNotADirectory(_) => "workspace/not-a-directory",
            Self::WorkspaceNotWritable(_) => "workspace/not-writable",
            Self::WorkspacePathUnavailable(_) => "workspace/path-unavailable",
            Self::DuplicateWorkspacePath(_) => "workspace/duplicate-path",
            Self::WorkspaceInUse(_) => "workspace/has-sessions",
            Self::WorkspaceReassignTargetInvalid(_) => "workspace/reassign-target-invalid",
            Self::DefaultWorkspace(_) => "workspace/default",
            Self::ConfirmRequired(_) => "workspace/confirm-required",
            Self::InvalidWorkspace(_) => "workspace/invalid",
            Self::Storage(_) => "storage/error",
            Self::Invalid(_) => "session/invalid",
        }
    }
}

/// Replaceable workspace store.
pub trait WorkspaceStore: Send + Sync {
    /// List workspaces, default first then creation order.
    fn list_workspaces(&self) -> Result<Vec<Workspace>, SessionError>;
    /// Register an existing local directory as a workspace.
    fn create_workspace(&self, path: &str, title: &str) -> Result<Workspace, SessionError>;
    /// Rename a workspace title (path/id unchanged).
    fn rename_workspace(&self, workspace_id: &str, title: &str) -> Result<Workspace, SessionError>;
    /// Make one workspace the default; returns the updated list.
    fn set_default_workspace(&self, workspace_id: &str) -> Result<Vec<Workspace>, SessionError>;
    /// Remove a workspace from the registry; conversations must be reassigned first.
    fn remove_workspace(
        &self,
        workspace_id: &str,
        reassign_to: Option<&str>,
        confirm: bool,
    ) -> Result<RemoveWorkspaceResult, SessionError>;
}

/// Replaceable session store (workspace-scoped).
pub trait SessionStore: WorkspaceStore + Send + Sync {
    /// Create a conversation inside a workspace.
    fn create_conversation_in(
        &self,
        workspace_id: &str,
        title: &str,
    ) -> Result<Conversation, SessionError>;
    /// List conversations of one workspace, newest first.
    fn list_conversations_in(&self, workspace_id: &str) -> Result<Vec<Conversation>, SessionError>;
    /// Append one message.
    fn append_message(&self, message: &Message) -> Result<(), SessionError>;
    /// Load messages of a conversation, verifying workspace ownership.
    fn messages_in(
        &self,
        workspace_id: &str,
        conversation_id: &str,
    ) -> Result<Vec<Message>, SessionError>;

    /// Rename one conversation (non-empty title).
    ///
    /// Default implementation keeps third-party/test stores compiling (Phase 2b
    /// additive trait methods, same convention as `store-session-sqlite`).
    fn rename_conversation(
        &self,
        _workspace_id: &str,
        _conversation_id: &str,
        _title: &str,
    ) -> Result<Conversation, SessionError> {
        Err(SessionError::Invalid(
            "rename_conversation unsupported".to_owned(),
        ))
    }

    /// Pin/unpin one conversation (orders it before unpinned ones).
    fn set_pinned(
        &self,
        _workspace_id: &str,
        _conversation_id: &str,
        _pinned: bool,
    ) -> Result<Conversation, SessionError> {
        Err(SessionError::Invalid("set_pinned unsupported".to_owned()))
    }

    /// Archive/unarchive one conversation (hide from the default list; keep data).
    fn set_archived(
        &self,
        _workspace_id: &str,
        _conversation_id: &str,
        _archived: bool,
    ) -> Result<Conversation, SessionError> {
        Err(SessionError::Invalid("set_archived unsupported".to_owned()))
    }

    /// Fork one conversation into the same workspace (messages copied).
    ///
    /// Default implementation is unsupported; the SQLite store overrides it.
    fn fork_conversation(
        &self,
        _workspace_id: &str,
        _conversation_id: &str,
        _title: Option<&str>,
    ) -> Result<Conversation, SessionError> {
        Err(SessionError::Invalid(
            "fork_conversation unsupported".to_owned(),
        ))
    }
}
