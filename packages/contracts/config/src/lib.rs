//! App settings / local provider config contract (minimal M1 subset).
//!
//! @capability contract.config
#![forbid(unsafe_code)]

use std::sync::{Mutex, OnceLock};

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

/// Tool execution and approval-wait timeouts for the local agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct ToolSettings {
    /// Per-tool-call timeout in milliseconds (1000..=600000).
    pub tool_timeout_ms: u64,
    /// Approval wait timeout in milliseconds (1000..=1800000).
    pub approval_timeout_ms: u64,
}

impl ToolSettings {
    /// Smallest accepted tool timeout.
    pub const MIN_TOOL_TIMEOUT_MS: u64 = 1_000;
    /// Largest accepted tool timeout.
    pub const MAX_TOOL_TIMEOUT_MS: u64 = 600_000;
    /// Smallest accepted approval timeout.
    pub const MIN_APPROVAL_TIMEOUT_MS: u64 = 1_000;
    /// Largest accepted approval timeout.
    pub const MAX_APPROVAL_TIMEOUT_MS: u64 = 1_800_000;

    /// Validate the configured ranges.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Invalid`] naming the offending field.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !(Self::MIN_TOOL_TIMEOUT_MS..=Self::MAX_TOOL_TIMEOUT_MS).contains(&self.tool_timeout_ms)
        {
            return Err(ConfigError::Invalid(format!(
                "tools.tool_timeout_ms must be within 1000..=600000 (got {})",
                self.tool_timeout_ms
            )));
        }
        if !(Self::MIN_APPROVAL_TIMEOUT_MS..=Self::MAX_APPROVAL_TIMEOUT_MS)
            .contains(&self.approval_timeout_ms)
        {
            return Err(ConfigError::Invalid(format!(
                "tools.approval_timeout_ms must be within 1000..=1800000 (got {})",
                self.approval_timeout_ms
            )));
        }
        Ok(())
    }

    /// The tool timeout clamped to the accepted range (runtime defense).
    #[must_use]
    pub fn tool_timeout(&self) -> u64 {
        self.tool_timeout_ms
            .clamp(Self::MIN_TOOL_TIMEOUT_MS, Self::MAX_TOOL_TIMEOUT_MS)
    }

    /// The approval timeout clamped to the accepted range (runtime defense).
    #[must_use]
    pub fn approval_timeout(&self) -> u64 {
        self.approval_timeout_ms
            .clamp(Self::MIN_APPROVAL_TIMEOUT_MS, Self::MAX_APPROVAL_TIMEOUT_MS)
    }
}

impl Default for ToolSettings {
    fn default() -> Self {
        Self {
            tool_timeout_ms: 120_000,
            approval_timeout_ms: 300_000,
        }
    }
}

/// Workspace schedule auto-run settings (`docs/19`/`docs/38`).
///
/// Auto-run starts scheduled prompts with no human in the loop, so it is
/// **disabled by default**; the desktop executor always starts but idles while
/// disabled. `scan_seconds` is the poll period for due schedules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct ScheduleSettings {
    /// Whether the desktop executes due `.zeokeel/schedules.json` entries.
    pub enabled: bool,
    /// Schedule scan period in seconds (`5..=3600`; default 10).
    #[schemars(range(min = 5, max = 3600))]
    pub scan_seconds: u64,
}

impl ScheduleSettings {
    /// Smallest accepted scan period.
    pub const MIN_SCAN_SECONDS: u64 = 5;
    /// Largest accepted scan period.
    pub const MAX_SCAN_SECONDS: u64 = 3_600;

    /// Validate the scan period.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Invalid`] when `scan_seconds` is out of range.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !(Self::MIN_SCAN_SECONDS..=Self::MAX_SCAN_SECONDS).contains(&self.scan_seconds) {
            return Err(ConfigError::Invalid(format!(
                "agent.schedule.scan_seconds must be within 5..=3600 (got {})",
                self.scan_seconds
            )));
        }
        Ok(())
    }

    /// The scan period clamped to the accepted range (runtime defense).
    #[must_use]
    pub fn effective_scan_seconds(&self) -> u64 {
        self.scan_seconds
            .clamp(Self::MIN_SCAN_SECONDS, Self::MAX_SCAN_SECONDS)
    }
}

impl Default for ScheduleSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            scan_seconds: 10,
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
    /// Workspace schedule auto-run (disabled by default).
    pub schedule: ScheduleSettings,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            max_steps: 50,
            completion: CompletionSettings::default(),
            memory: MemorySettings::default(),
            hooks: HookSettings::default(),
            schedule: ScheduleSettings::default(),
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
        self.schedule.validate()?;
        Ok(())
    }
}

/// Personal-edition embedding/vector provider settings (`cap-vector`).
///
/// Custom settings are only valid for personal edition: enterprise AI must go
/// through the gateway (C7), and the generated glue never configures a local
/// embedding provider there. `api_key` is write-only in the UI (`settings.get`
/// masks it, like the chat provider key).
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct EmbeddingSettings {
    /// OpenAI-compatible API root, e.g. `https://api.openai.com/v1`.
    pub base_url: String,
    /// Bearer key (write-only: masked by `settings.get`).
    pub api_key: String,
    /// Embedding model id (e.g. `text-embedding-3-small`).
    pub model: String,
    /// Requested vector width (1..=65536), when supported by the endpoint.
    #[schemars(range(min = 1, max = 65536))]
    pub dims: Option<u64>,
    /// Persistence directory for `vector-index.json`; empty = memory-only.
    pub dir: String,
}

impl std::fmt::Debug for EmbeddingSettings {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = formatter.debug_struct("EmbeddingSettings");
        debug
            .field("base_url", &self.base_url)
            .field(
                "api_key",
                &if self.api_key.is_empty() {
                    ""
                } else {
                    "[redacted]"
                },
            )
            .field("model", &self.model)
            .field("dims", &self.dims)
            .field("dir", &self.dir)
            .finish()
    }
}

impl EmbeddingSettings {
    /// Smallest accepted vector width.
    pub const MIN_DIMS: u64 = 1;
    /// Largest accepted vector width.
    pub const MAX_DIMS: u64 = 65_536;

    /// Validate the vector width and API scheme.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Invalid`] for out-of-range `dims` or a non-http(s)
    /// `base_url`.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if let Some(dims) = self.dims {
            if !(Self::MIN_DIMS..=Self::MAX_DIMS).contains(&dims) {
                return Err(ConfigError::Invalid(format!(
                    "embedding.dims must be within 1..=65536 (got {dims})"
                )));
            }
        }
        let base_url = self.base_url.trim();
        if !base_url.is_empty()
            && !(base_url.starts_with("http://") || base_url.starts_with("https://"))
        {
            return Err(ConfigError::Invalid(format!(
                "embedding.base_url must start with http:// or https:// (got `{base_url}`)"
            )));
        }
        Ok(())
    }

    /// Whether a settings override carries enough data to configure a provider.
    ///
    /// A blank API key is the UI's masked value and means "no override" (keep
    /// whatever was configured from the environment at boot).
    #[must_use]
    pub fn is_configured(&self) -> bool {
        !self.api_key.trim().is_empty()
    }
}

/// Personal-edition media capability settings (`cap-image`/`cap-audio`/
/// `cap-transcription`/`cap-rerank`).
///
/// One OpenAI/Voyage-compatible endpoint and key drive all four kinds; each kind
/// has its own optional model id (empty = the capability's default model). Like
/// [`EmbeddingSettings`], this is personal-edition only (C7): enterprise AI must
/// go through the gateway; the generated glue never configures local providers
/// there.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct MediaSettings {
    /// OpenAI-compatible API root (empty = each kind's provider default).
    pub base_url: String,
    /// Bearer key shared by the four media kinds (write-only: masked).
    pub api_key: String,
    /// Image-generation model id (empty = capability default).
    pub image_model: String,
    /// Speech-synthesis model id (empty = capability default).
    pub audio_model: String,
    /// Transcription model id (empty = capability default).
    pub transcription_model: String,
    /// Rerank model id (empty = capability default).
    pub rerank_model: String,
}

impl std::fmt::Debug for MediaSettings {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = formatter.debug_struct("MediaSettings");
        debug
            .field("base_url", &self.base_url)
            .field(
                "api_key",
                &if self.api_key.is_empty() {
                    ""
                } else {
                    "[redacted]"
                },
            )
            .field("image_model", &self.image_model)
            .field("audio_model", &self.audio_model)
            .field("transcription_model", &self.transcription_model)
            .field("rerank_model", &self.rerank_model)
            .finish()
    }
}

impl MediaSettings {
    /// Validate the shared API scheme.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Invalid`] for a non-http(s) non-empty `base_url`.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let base_url = self.base_url.trim();
        if !base_url.is_empty()
            && !(base_url.starts_with("http://") || base_url.starts_with("https://"))
        {
            return Err(ConfigError::Invalid(format!(
                "media.base_url must start with http:// or https:// (got `{base_url}`)"
            )));
        }
        Ok(())
    }

    /// Whether a settings override carries an API key (blank = masked/no override).
    #[must_use]
    pub fn is_configured(&self) -> bool {
        !self.api_key.trim().is_empty()
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
    /// Tool execution / approval timeouts (backward compatible default).
    #[serde(default)]
    pub tools: ToolSettings,
    /// Personal-edition embedding/vector provider (backward compatible default).
    #[serde(default)]
    pub embedding: EmbeddingSettings,
    /// Personal-edition media capabilities (backward compatible default).
    #[serde(default)]
    pub media: MediaSettings,
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
            tools: ToolSettings::default(),
            embedding: EmbeddingSettings::default(),
            media: MediaSettings::default(),
        }
    }
}

impl AppSettings {
    /// Validate every configurable section before persisting/applying it.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Invalid`] naming the offending field.
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.agent.validate()?;
        self.tools.validate()?;
        self.embedding.validate()?;
        self.media.validate()
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

/// Non-secret settings snapshot published to the local agent runtime.
///
/// Deliberately contains no `ProviderConfig`: provider credentials/base URLs
/// are host/secret-store concerns and must never be readable by every package
/// linked against the config contract (review round-3 P2). Only the sections
/// the runtime needs on each turn are published.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct RuntimeSettings {
    /// Runtime agent settings, when explicitly saved (non-default).
    ///
    /// `None` means "not published": the runtime keeps the env/default
    /// fallback for the agent section instead of overriding it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentSettings>,
    /// Tool execution / approval-wait timeouts, when explicitly saved.
    ///
    /// `None` means "not published": the runtime keeps the env/default
    /// fallback for the tools section.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolSettings>,
}

impl RuntimeSettings {
    /// Validate both published sections.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Invalid`] when agent or tool settings are out of range.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if let Some(agent) = &self.agent {
            agent.validate()?;
        }
        if let Some(tools) = &self.tools {
            tools.validate()?;
        }
        Ok(())
    }
}

impl From<&AppSettings> for RuntimeSettings {
    fn from(settings: &AppSettings) -> Self {
        Self {
            agent: Some(settings.agent.clone()),
            tools: Some(settings.tools),
        }
    }
}

/// Host → runtime seam for the settings snapshot the local agent should use.
///
/// The desktop owns the persisted store and the agent runtime owns the turn
/// loop; both depend on this contract, so the resolved **non-secret**
/// [`RuntimeSettings`] is published here (no package-to-package dependency).
/// The runtime reads the last published snapshot on each turn. `None` means
/// "never published" so the runtime keeps its `ZEOKEEL_*` environment
/// fallbacks (precedence: explicitly saved settings > env > built-in default).
///
/// NOTE (architecture): this is mutable runtime state on a contract crate.
/// A future `zk-kit-agent-config` host kit should own it; tracked as a
/// follow-up for the Lead/ADR (same class as the contract-agent hook registry).
static RUNTIME_SETTINGS: OnceLock<Mutex<Option<RuntimeSettings>>> = OnceLock::new();

fn runtime_slot() -> &'static Mutex<Option<RuntimeSettings>> {
    RUNTIME_SETTINGS.get_or_init(|| Mutex::new(None))
}

/// Publish the resolved **non-secret** settings snapshot for the local agent
/// runtime (never the provider config / API key).
///
/// # Errors
///
/// [`ConfigError::Invalid`] when the agent or tool sections fail validation;
/// the previous snapshot is kept.
pub fn set_runtime_settings(settings: &RuntimeSettings) -> Result<(), ConfigError> {
    settings.validate()?;
    let mut slot = runtime_slot()
        .lock()
        .map_err(|_| ConfigError::Storage("runtime settings registry poisoned".to_owned()))?;
    *slot = Some(settings.clone());
    Ok(())
}

/// The published non-secret settings snapshot, or `None` when never published.
///
/// # Errors
///
/// [`ConfigError::Storage`] when the registry lock is poisoned.
pub fn runtime_settings() -> Result<Option<RuntimeSettings>, ConfigError> {
    runtime_slot()
        .lock()
        .map(|slot| slot.clone())
        .map_err(|_| ConfigError::Storage("runtime settings registry poisoned".to_owned()))
}

/// Clear the published snapshot (logout/tests).
///
/// # Errors
///
/// [`ConfigError::Storage`] when the registry lock is poisoned.
pub fn clear_runtime_settings() -> Result<(), ConfigError> {
    let mut slot = runtime_slot()
        .lock()
        .map_err(|_| ConfigError::Storage("runtime settings registry poisoned".to_owned()))?;
    *slot = None;
    Ok(())
}
