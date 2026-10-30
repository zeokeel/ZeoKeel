//! Package identity contract (minimal M0 subset).
//!
//! @capability contract.package
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Stable identity of a composed package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PackageDescriptor {
    /// Package id, e.g. `zeokeel/tool-fs`.
    pub id: String,
    /// Package version (semver).
    pub version: String,
    /// Package kind, e.g. `capability`.
    pub kind: String,
    /// Roles provided by this package.
    pub roles: Vec<String>,
}

/// Implemented by every composed package.
pub trait Package: Send + Sync {
    /// Immutable descriptor used by the host for introspection.
    fn descriptor(&self) -> PackageDescriptor;
}
