//! Personal update check via GitHub Releases `latest.json` (ADR-041).
#![forbid(unsafe_code)]

use zk_contract_update::{ReleaseAsset, ReleaseInfo};

const DEFAULT_FEED: &str = "https://github.com/zeokeel/ZeoKeel/releases/latest/download/latest.json";

/// Closed update-check error set.
#[derive(Debug)]
pub enum UpdateError {
    /// Network/HTTP failure.
    Transport(String),
    /// Feed JSON malformed.
    Protocol(String),
    /// Invalid version string.
    Version(String),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(error) => write!(f, "update transport: {error}"),
            Self::Protocol(error) => write!(f, "update protocol: {error}"),
            Self::Version(error) => write!(f, "update version: {error}"),
        }
    }
}
impl std::error::Error for UpdateError {}

/// Result of one check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCheck {
    pub current: String,
    pub available: bool,
    pub release: Option<ReleaseInfo>,
}

impl UpdateCheck {
    /// Asset for this platform, if the release has one.
    #[must_use]
    pub fn asset_for(&self, platform: &str) -> Option<&ReleaseAsset> {
        self.release.as_ref()?.assets.iter().find(|asset| asset.platform == platform)
    }
}

/// Check the personal feed; no download/install side effects.
pub fn check(current: &str, platform: &str) -> Result<UpdateCheck, UpdateError> {
    let feed = std::env::var("ZEOKEEL_UPDATE_FEED").unwrap_or_else(|_| DEFAULT_FEED.to_owned());
    let current_version = semver::Version::parse(current).map_err(|error| UpdateError::Version(error.to_string()))?;
    let response = reqwest::blocking::get(&feed).map_err(|error| UpdateError::Transport(error.to_string()))?;
    if !response.status().is_success() {
        return Err(UpdateError::Transport(format!("feed HTTP {}", response.status().as_u16())));
    }
    let body = response.text().map_err(|error| UpdateError::Transport(error.to_string()))?;
    let release: ReleaseInfo = serde_json::from_str(&body).map_err(|error| UpdateError::Protocol(error.to_string()))?;
    let latest = semver::Version::parse(&release.version).map_err(|error| UpdateError::Version(error.to_string()))?;
    let available = latest > current_version;
    let asset = release.assets.iter().find(|asset| asset.platform == platform).is_some();
    Ok(UpdateCheck { current: current.to_owned(), available: available && asset, release: Some(release) })
}
