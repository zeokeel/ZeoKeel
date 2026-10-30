//! Server-side domain contract marker (role `srv.http` and friends).
//!
//! @capability contract.server
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Stable server module descriptor exposed by `srv.*` packages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ServerModule {
    /// Namespaced module id (e.g. `srv/http`).
    pub id: String,
    /// Human-readable version.
    pub version: String,
}

impl ServerModule {
    /// Build a descriptor.
    #[must_use]
    pub fn new(id: &str, version: &str) -> Self {
        Self {
            id: id.to_owned(),
            version: version.to_owned(),
        }
    }
}
