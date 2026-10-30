//! Admin UI contract seam (`admin.shell`, `admin.page.*`).
//!
//! @capability contract.admin-ui
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};

/// Admin page descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminPage {
    pub id: String,
    pub title: String,
}
