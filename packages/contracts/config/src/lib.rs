//! App settings / local provider config contract (minimal M1 subset).
//!
//! @capability contract.config
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Provider kind (personal edition local providers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
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

/// How the model may choose tools for a turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default)]
#[serde(tag = "mode", content = "tools", rename_all = "snake_case")]
pub enum ToolChoiceSetting {
    /// The model decides (default).
    #[default]
    Auto,
    /// No tool may be called.
    None,
    /// At least one tool must be called.
    Required,
    /// Only the listed tools may be called.
    Specific(Vec<String>),
}

/// Sampling / generation parameters for the local agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema, Default)]
#[serde(default)]
pub struct CompletionSettings {
    /// Sampling temperature (`0.0..=2.0`); omitted keeps the provider default.
    pub temperature: Option<f64>,
    /// Output-token cap; omitted keeps the provider default.
    pub max_tokens: Option<u64>,
    /// Nucleus sampling (`0.0..=1.0`); omitted keeps the provider default.
    pub top_p: Option<f64>,
    /// Sampling seed for reproducible runs.
    pub seed: Option<u64>,
    /// Stop sequences.
    pub stop: Vec<String>,
    /// Tool-selection policy.
    pub tool_choice: ToolChoiceSetting,
    /// Whether the provider may run tool calls in parallel.
    pub parallel_tool_calls: Option<bool>,
    /// Portable reasoning effort: `minimal|low|medium|high|xhigh|max`.
    pub reasoning_effort: Option<String>,
    /// Prompt-cache retention: `short|long`.
    pub prompt_cache: Option<String>,
}

/// Multi-turn conversation memory for the local agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct MemorySettings {
    /// Whether conversation memory is enabled.
    pub enabled: bool,
    /// Memory directory; empty means `$ZEOKEEL_HOME/agent-memory`.
    pub dir: String,
}

impl Default for MemorySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            dir: String::new(),
        }
    }
}

/// Observable agent lifecycle hooks configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct HookSettings {
    /// Whether lifecycle hook events are emitted.
    pub enabled: bool,
    /// Optional JSONL file that receives the hook events (append-only).
    pub log: String,
}

impl Default for HookSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            log: String::new(),
        }
    }
}

/// Runtime-configurable local agent settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct AgentSettings {
    /// Maximum model/tool steps per turn (`1..=200`; default 50).
    pub max_steps: usize,
    /// Sampling / generation parameters.
    pub completion: CompletionSettings,
    /// Multi-turn conversation memory.
    pub memory: MemorySettings,
    /// Observable lifecycle hooks.
    pub hooks: HookSettings,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            max_steps: 50,
            completion: CompletionSettings::default(),
            memory: MemorySettings::default(),
            hooks: HookSettings::default(),
        }
    }
}

impl AgentSettings {
    /// Validate ranges and closed enums before the runtime applies the settings.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Invalid`] naming the offending field.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !(1..=200).contains(&self.max_steps) {
            return Err(ConfigError::Invalid(format!(
                "agent.max_steps must be within 1..=200 (got {})",
                self.max_steps
            )));
        }
        if let Some(temperature) = self.completion.temperature {
            if !(0.0..=2.0).contains(&temperature) {
                return Err(ConfigError::Invalid(format!(
                    "agent.completion.temperature must be within 0.0..=2.0 (got {temperature})"
                )));
            }
        }
        if self.completion.max_tokens == Some(0) {
            return Err(ConfigError::Invalid(
                "agent.completion.max_tokens must be >= 1".to_owned(),
            ));
        }
        if let Some(top_p) = self.completion.top_p {
            if !(0.0..=1.0).contains(&top_p) {
                return Err(ConfigError::Invalid(format!(
                    "agent.completion.top_p must be within 0.0..=1.0 (got {top_p})"
                )));
            }
        }
        if self
            .completion
            .stop
            .iter()
            .any(|item| item.trim().is_empty())
        {
            return Err(ConfigError::Invalid(
                "agent.completion.stop must not contain empty entries".to_owned(),
            ));
        }
        if let ToolChoiceSetting::Specific(tools) = &self.completion.tool_choice {
            if tools.is_empty() || tools.iter().any(|tool| tool.trim().is_empty()) {
                return Err(ConfigError::Invalid(
                    "agent.completion.tool_choice=specific requires non-empty tool names"
                        .to_owned(),
                ));
            }
        }
        if let Some(effort) = &self.completion.reasoning_effort {
            if !matches!(
                effort.as_str(),
                "minimal" | "low" | "medium" | "high" | "xhigh" | "max"
            ) {
                return Err(ConfigError::Invalid(format!(
                    "agent.completion.reasoning_effort must be minimal|low|medium|high|xhigh|max (got {effort})"
                )));
            }
        }
        if let Some(cache) = &self.completion.prompt_cache {
            if !matches!(cache.as_str(), "short" | "long") {
                return Err(ConfigError::Invalid(format!(
                    "agent.completion.prompt_cache must be short|long (got {cache})"
                )));
            }
        }
        if !self.memory.dir.is_empty() && self.memory.dir.trim().is_empty() {
            return Err(ConfigError::Invalid(
                "agent.memory.dir must be empty or a real path".to_owned(),
            ));
        }
        if !self.hooks.log.is_empty() && self.hooks.log.trim().is_empty() {
            return Err(ConfigError::Invalid(
                "agent.hooks.log must be empty or a real path".to_owned(),
            ));
        }
        Ok(())
    }
}

/// App settings persisted locally.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
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
    /// Runtime-configurable local agent settings (backward compatible default).
    #[serde(default)]
    pub agent: AgentSettings,
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
            agent: AgentSettings::default(),
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
