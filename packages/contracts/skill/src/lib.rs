//! Client-facing skill catalog contract (`ui.slot.skills`).
//!
//! @capability contract.skill
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};

/// Catalog entry returned by `GET /api/client/v1/skills`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SkillCatalogEntry {
    pub name: String,
    pub version: String,
    pub title: String,
    pub category: String,
    pub sha256: String,
    pub min_client_version: String,
    pub enforced: bool,
    pub can_install: bool,
}

/// Install receipt sent by the client (operational fact only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SkillInstallReceipt {
    pub name: String,
    pub version: String,
    pub tree_sha256: String,
    pub source: String,
}
