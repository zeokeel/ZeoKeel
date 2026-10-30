//! Content aggregate contract (`srv.content`).
//!
//! @capability contract.content
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};

/// Content kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContentKind { Skill, App }

/// Content lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContentStatus { Pending, Approved, Rejected, Delisted }

/// Visibility scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Visibility { All, Selected }

/// One immutable release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ContentRelease {
    pub id: String,
    pub content_id: String,
    pub version: String,
    pub sha256: String,
    pub archive_size: u64,
    #[serde(default)]
    pub changelog: String,
}

/// Content record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Content {
    pub id: String,
    pub kind: ContentKind,
    pub name: String,
    pub owner_id: String,
    pub status: ContentStatus,
    pub visibility: Visibility,
    pub current_release_id: Option<String>,
}

/// Grant (all or user).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ContentGrant {
    pub grantee: String,
    pub can_install: bool,
}
