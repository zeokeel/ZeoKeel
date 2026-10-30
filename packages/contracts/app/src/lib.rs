//! WASM app contracts (M4): catalog/open/frame/manifest.
#![forbid(unsafe_code)]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// App visibility/access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AppAccess { All, Selected }

/// App catalog item (docs/04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AppItem {
    pub app_id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub icon: String,
    pub version: String,
    pub access: AppAccess,
    #[serde(default)]
    pub ui: bool,
}

/// Open response (headers are authoritative; body is informational).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AppOpen {
    pub app_id: String,
    pub version: String,
    pub ui_manifest: String,
}

/// App request envelope (docs/04 §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AppRequest {
    pub abi: String,
    pub app_id: String,
    #[serde(default)]
    pub version: Option<String>,
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub headers: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub body_b64: String,
    pub request_id: String,
}

/// App response envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AppResponse {
    pub status: u16,
    #[serde(default)]
    pub headers: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub body_b64: String,
}

/// Release manifest uploaded with app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AppManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub icon: String,
    pub db_schema_version: u32,
    #[serde(default)]
    pub db_migrations: Vec<AppMigration>,
    #[serde(default)]
    pub ui: Vec<String>,
}

/// One host-managed schema migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AppMigration { pub version: u32, pub sql: String }

/// Placeholder for future protocol constants.
pub const ABI: &str = "zeokeel-app/1";
