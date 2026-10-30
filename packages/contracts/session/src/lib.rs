//! Conversation/session persistence contract (minimal M1 subset).
//!
//! @capability contract.session
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One conversation (message thread).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conversation {
    /// Stable id (uuid/ulid string).
    pub id: String,
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

/// Closed error set for session storage.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SessionError {
    /// Conversation not found.
    #[error("conversation not found: {0}")]
    NotFound(String),
    /// Storage failure.
    #[error("session storage error: {0}")]
    Storage(String),
    /// Invalid input.
    #[error("session invalid input: {0}")]
    Invalid(String),
}

/// Replaceable session store (SQLite implementation lives in a package).
pub trait SessionStore: Send + Sync {
    /// Create a conversation.
    fn create_conversation(&self, title: &str) -> Result<Conversation, SessionError>;
    /// List conversations, newest first.
    fn list_conversations(&self) -> Result<Vec<Conversation>, SessionError>;
    /// Append one message.
    fn append_message(&self, message: &Message) -> Result<(), SessionError>;
    /// Load messages in chronological order.
    fn messages(&self, conversation_id: &str) -> Result<Vec<Message>, SessionError>;
}
