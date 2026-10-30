//! Update-check contract (`update.client`).
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};

/// One downloadable release artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub platform: String,
    pub url: String,
    pub sha256: String,
}

/// Release feed payload (`latest.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub version: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}
