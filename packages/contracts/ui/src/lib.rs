//! UI contribution contract (minimal M1 subset).
//!
//! @capability contract.ui
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One static UI contribution declared by a UI package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiContribution {
    /// Slot id, e.g. `ui.chat`.
    pub slot: String,
    /// Component id, e.g. `chat.thread`.
    pub component: String,
    /// Order inside a multi-slot (lower first).
    #[serde(default)]
    pub order: i32,
}

/// Closed error set for UI contribution validation.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum UiError {
    /// Slot is not in the frozen slot set.
    #[error("unknown slot: {0}")]
    UnknownSlot(String),
    /// Component id duplicated.
    #[error("duplicate component: {0}")]
    DuplicateComponent(String),
}
