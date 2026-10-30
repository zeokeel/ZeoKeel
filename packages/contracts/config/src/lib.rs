//! App settings / local provider config contract (minimal M1 subset).
//!
//! @capability contract.config
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Provider kind (personal edition local providers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// Deterministic offline mock.
    Mock,
    /// OpenAI-compatible endpoint (DeepSeek/OpenAI/vLLM...).
    OpenAiCompatible,
    /// Local Ollama daemon.
    Ollama,
}

/// Local provider configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider kind.
    pub kind: ProviderKind,
    /// Base URL (required for non-mock).
    #[serde(default)]
    pub base_url: String,
    /// Model name.
    #[serde(default)]
    pub model: String,
    /// API key reference (never the raw key in logs/DB; M1 stores locally).
    #[serde(default)]
    pub api_key: String,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            kind: ProviderKind::Mock,
            base_url: String::new(),
            model: "mock".to_owned(),
            api_key: String::new(),
        }
    }
}

/// App settings persisted locally.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    /// Schema version.
    pub schema: u32,
    /// Provider config.
    pub provider: ProviderConfig,
    /// Theme: `auto` | `light` | `dark`.
    pub theme: String,
    /// Locale: `zh-CN` | `en-US`.
    pub locale: String,
    /// Auto-approve read-only tools (M1 default true for fs.read).
    pub auto_approve_read_only: bool,
    /// Enterprise login server URL (personal edition: empty).
    #[serde(default)]
    pub server_url: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema: 1,
            provider: ProviderConfig::default(),
            theme: "auto".to_owned(),
            locale: "zh-CN".to_owned(),
            auto_approve_read_only: true,
            server_url: String::new(),
        }
    }
}

/// Closed error set for settings storage.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConfigError {
    /// Storage failure.
    #[error("config storage error: {0}")]
    Storage(String),
    /// Invalid value.
    #[error("config invalid: {0}")]
    Invalid(String),
}

/// Replaceable settings store (JSON file in M1, keychain later).
pub trait SettingsStore: Send + Sync {
    /// Load settings (defaults if missing).
    fn load(&self) -> Result<AppSettings, ConfigError>;
    /// Persist settings.
    fn save(&self, settings: &AppSettings) -> Result<(), ConfigError>;
}
