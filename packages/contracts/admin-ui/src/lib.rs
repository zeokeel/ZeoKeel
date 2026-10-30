//! Admin UI contract seam (`admin.shell`, `admin.page.*`).
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};

/// Admin page descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminPage {
    pub id: String,
    pub title: String,
}
