//! Conversation/workspace persistence contract (M1 workspace-scoped subset).
//!
//! @capability contract.session
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One workspace: a local directory that owns conversations and agent cwd.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveWorkspaceResult {
    /// Always true when the call succeeds.
    pub removed: bool,
    /// Number of conversations moved to `reassign_to`.
    pub moved_sessions: u64,
}

/// One conversation (message thread), always owned by exactly one workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// One message inside a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    #[error("session/workspace-mismatch: conversation {conversation} is not in workspace {workspace}")]
    WorkspaceMismatch {
        /// Conversation id.
        conversation: String,
        /// Expected workspace id.
        workspace: String,
    },
    /// Workspace path is unavailable (missing/not a directory/not writable).
    #[error("workspace path unavailable: {0}")]
    WorkspacePathUnavailable(String),
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
pub trait SessionStore: Send + Sync {
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

    /// Compatibility: create in the default workspace. **Deprecated; T4 removes it.**
    #[deprecated = "use create_conversation_in(workspace_id, title)"]
    fn create_conversation(&self, title: &str) -> Result<Conversation, SessionError>;
    /// Compatibility: list default workspace conversations. **Deprecated; T4 removes it.**
    #[deprecated = "use list_conversations_in(workspace_id)"]
    fn list_conversations(&self) -> Result<Vec<Conversation>, SessionError>;
    /// Compatibility: read messages without explicit workspace. **Deprecated; T4 removes it.**
    #[deprecated = "use messages_in(workspace_id, conversation_id)"]
    fn messages(&self, conversation_id: &str) -> Result<Vec<Message>, SessionError>;
}
