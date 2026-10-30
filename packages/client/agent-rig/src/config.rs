//! Runtime-configurable agent settings (completion / memory / hooks).
//!
//! The config contract (`AppSettings` in `zk-contract-config`) cannot carry
//! these fields while task-14 (T22 schemars) owns `packages/contracts/*`, so the
//! agent runtime accepts them here through two paths:
//!
//! 1. [`set_default_agent_config`] / [`default_agent_config`] — process-global
//!    default the desktop settings loader can install after reading
//!    `AppSettings` (see the exact schema reported to the Lead).
//! 2. [`crate::RigAgent::with_agent_config`] / [`crate::RigAgent::configure_agent`]
//!    — per-instance override (tests and future glue wiring).
//!
//! Environment variables are the last fallback, mainly for `scripts/agent-e2e.sh`
//! and headless runs. Precedence: **explicit config > env > built-in default**.
//!
//! Built-in defaults match `docs/38-agent-runtime.md`: provider defaults for
//! sampling (no temperature/max_tokens/top_p/seed is imposed), automatic tool
//! choice, serial tool calls (rig/provider default), no reasoning override, no
//! prompt-cache override; memory and hooks are enabled but only persist/observe
//! when a directory/log is configured.

use serde::{Deserialize, Serialize};
use zk_contract_config::{AgentSettings, ToolChoiceSetting, ToolSettings};

/// Multi-turn memory via `rig-memory` conversation persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryConfig {
    /// Whether conversation memory is enabled for this agent.
    pub enabled: bool,
    /// Directory holding the `rig-memory` conversation logs. Empty means
    /// `$ZEOKEEL_HOME/agent-memory`, then `$ZEOKEEL_AGENT_MEMORY_DIR` (legacy
    /// env), then no persistence (reported through the hook stream).
    pub dir: String,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dir: String::new(),
        }
    }
}

/// Observable lifecycle hooks (turn/tool/approval events) for hosts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HooksConfig {
    /// Whether lifecycle events are emitted to the installed [`crate::HookSink`].
    pub enabled: bool,
    /// Optional JSONL file that receives the same events (append-only).
    pub log: String,
}

impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log: String::new(),
        }
    }
}

/// How a tool may be chosen for a completion request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "mode", content = "tools", rename_all = "snake_case")]
pub enum ToolChoiceConfig {
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

/// Portable reasoning effort (rig `Effort`), lowest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    /// Least reasoning.
    Minimal,
    /// Low effort.
    Low,
    /// Medium effort.
    Medium,
    /// High effort.
    High,
    /// Extra-high effort.
    XHigh,
    /// Most reasoning.
    Max,
}

/// Prompt-cache retention to request from the provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptCache {
    /// The provider's short retention (typically minutes).
    Short,
    /// The provider's long retention (typically an hour or more).
    Long,
}

/// Sampling / generation parameters applied to every agent completion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CompletionConfig {
    /// Sampling temperature (`0.0..=2.0`); `None` keeps the provider default.
    pub temperature: Option<f64>,
    /// Output-token cap; `None` keeps the provider default.
    pub max_tokens: Option<u64>,
    /// Nucleus sampling (`0.0..=1.0`); `None` keeps the provider default.
    pub top_p: Option<f64>,
    /// Sampling seed for reproducible runs.
    pub seed: Option<u64>,
    /// Stop sequences.
    pub stop: Vec<String>,
    /// Tool-selection policy.
    pub tool_choice: ToolChoiceConfig,
    /// Whether the provider may run tool calls in parallel.
    pub parallel_tool_calls: Option<bool>,
    /// Reasoning effort override.
    pub reasoning_effort: Option<ReasoningEffort>,
    /// Prompt-cache retention override.
    pub prompt_cache: Option<PromptCache>,
}

impl Default for CompletionConfig {
    fn default() -> Self {
        Self {
            temperature: None,
            max_tokens: None,
            top_p: None,
            seed: None,
            stop: Vec::new(),
            tool_choice: ToolChoiceConfig::Auto,
            parallel_tool_calls: None,
            reasoning_effort: None,
            prompt_cache: None,
        }
    }
}

impl CompletionConfig {
    /// Validate ranges and shapes before the runtime applies the config.
    ///
    /// # Errors
    ///
    /// A human-readable message naming the offending field.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(temperature) = self.temperature {
            if !(0.0..=2.0).contains(&temperature) {
                return Err(format!(
                    "agent.completion.temperature must be within 0.0..=2.0 (got {temperature})"
                ));
            }
        }
        if self.max_tokens == Some(0) {
            return Err("agent.completion.max_tokens must be >= 1".to_owned());
        }
        if let Some(top_p) = self.top_p {
            if !(0.0..=1.0).contains(&top_p) {
                return Err(format!(
                    "agent.completion.top_p must be within 0.0..=1.0 (got {top_p})"
                ));
            }
        }
        if self.stop.iter().any(|item| item.trim().is_empty()) {
            return Err("agent.completion.stop must not contain empty entries".to_owned());
        }
        if let ToolChoiceConfig::Specific(tools) = &self.tool_choice {
            if tools.is_empty() {
                return Err(
                    "agent.completion.tool_choice=specific requires at least one tool name"
                        .to_owned(),
                );
            }
            if tools.iter().any(|tool| tool.trim().is_empty()) {
                return Err("agent.completion.tool_choice tool names must be non-empty".to_owned());
            }
        }
        Ok(())
    }

    /// Whether every setting is at its built-in default.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

/// All runtime-configurable agent settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentRuntimeConfig {
    /// Sampling / generation parameters.
    pub completion: CompletionConfig,
    /// Multi-turn conversation memory.
    pub memory: MemoryConfig,
    /// Observable lifecycle hooks.
    pub hooks: HooksConfig,
    /// Tool execution / approval-wait timeouts.
    pub tools: ToolSettings,
    /// Maximum model/tool steps per turn (1..=200).
    pub max_steps: usize,
}

impl Default for AgentRuntimeConfig {
    fn default() -> Self {
        Self {
            completion: CompletionConfig::default(),
            memory: MemoryConfig::default(),
            hooks: HooksConfig::default(),
            tools: ToolSettings::default(),
            max_steps: crate::MAX_STEPS,
        }
    }
}

impl AgentRuntimeConfig {
    /// Built-in defaults overlaid with `ZEOKEEL_AGENT_*` environment fallbacks.
    #[must_use]
    pub fn from_env() -> Self {
        let mut config = Self::default();
        if let Ok(value) = std::env::var("ZEOKEEL_AGENT_MAX_STEPS") {
            if let Ok(steps) = value.parse::<usize>() {
                config.max_steps = steps;
            }
        }
        config.completion.temperature = env_parse("ZEOKEEL_AGENT_TEMPERATURE");
        config.completion.max_tokens = env_parse("ZEOKEEL_AGENT_MAX_TOKENS");
        config.completion.top_p = env_parse("ZEOKEEL_AGENT_TOP_P");
        config.completion.seed = env_parse("ZEOKEEL_AGENT_SEED");
        if let Ok(value) = std::env::var("ZEOKEEL_AGENT_STOP") {
            config.completion.stop = value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_owned)
                .collect();
        }
        if let Ok(value) = std::env::var("ZEOKEEL_AGENT_TOOL_CHOICE") {
            config.completion.tool_choice = match value.trim().to_ascii_lowercase().as_str() {
                "none" => ToolChoiceConfig::None,
                "required" => ToolChoiceConfig::Required,
                _ => ToolChoiceConfig::Auto,
            };
        }
        config.completion.parallel_tool_calls = env_parse("ZEOKEEL_AGENT_PARALLEL_TOOL_CALLS");
        config.completion.reasoning_effort = std::env::var("ZEOKEEL_AGENT_REASONING_EFFORT")
            .ok()
            .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
                "minimal" => Some(ReasoningEffort::Minimal),
                "low" => Some(ReasoningEffort::Low),
                "medium" => Some(ReasoningEffort::Medium),
                "high" => Some(ReasoningEffort::High),
                "xhigh" => Some(ReasoningEffort::XHigh),
                "max" => Some(ReasoningEffort::Max),
                _ => None,
            });
        config.completion.prompt_cache =
            std::env::var("ZEOKEEL_AGENT_PROMPT_CACHE")
                .ok()
                .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
                    "short" => Some(PromptCache::Short),
                    "long" => Some(PromptCache::Long),
                    _ => None,
                });
        if let Ok(value) = std::env::var("ZEOKEEL_AGENT_MEMORY_ENABLED") {
            config.memory.enabled = value != "0";
        }
        if let Ok(value) = std::env::var("ZEOKEEL_AGENT_MEMORY_DIR") {
            config.memory.dir = value;
        }
        if let Ok(value) = std::env::var("ZEOKEEL_AGENT_HOOKS_ENABLED") {
            config.hooks.enabled = value != "0";
        }
        if let Ok(value) = std::env::var("ZEOKEEL_AGENT_HOOKS_LOG") {
            config.hooks.log = value;
        }
        if let Ok(value) = std::env::var("ZEOKEEL_TOOL_TIMEOUT_MS") {
            if let Ok(timeout) = value.parse::<u64>() {
                config.tools.tool_timeout_ms = timeout;
            }
        }
        if let Ok(value) = std::env::var("ZEOKEEL_APPROVAL_TIMEOUT_MS") {
            if let Ok(timeout) = value.parse::<u64>() {
                config.tools.approval_timeout_ms = timeout;
            }
        }
        config.max_steps = config.max_steps.clamp(1, 200);
        config
    }

    /// Resolve the memory directory, if memory is enabled and a location is
    /// configured. Returns `None` when persistence cannot be enabled, so the
    /// caller can report it rather than silently dropping context.
    #[must_use]
    pub fn memory_dir(&self) -> Option<std::path::PathBuf> {
        if !self.memory.enabled {
            return None;
        }
        if !self.memory.dir.trim().is_empty() {
            return Some(std::path::PathBuf::from(self.memory.dir.trim()));
        }
        if let Some(dir) = std::env::var_os("ZEOKEEL_AGENT_MEMORY_DIR") {
            if !std::path::Path::new(&dir).as_os_str().is_empty() {
                return Some(std::path::PathBuf::from(dir));
            }
        }
        std::env::var_os("ZEOKEEL_HOME")
            .filter(|home| !home.is_empty())
            .map(|home| std::path::PathBuf::from(home).join("agent-memory"))
    }

    /// Validate every section.
    ///
    /// # Errors
    ///
    /// A human-readable message naming the offending field.
    pub fn validate(&self) -> Result<(), String> {
        self.completion.validate()?;
        self.tools.validate().map_err(|error| error.to_string())?;
        if !(1..=200).contains(&self.max_steps) {
            return Err(format!(
                "agent.max_steps must be within 1..=200 (got {})",
                self.max_steps
            ));
        }
        if !self.hooks.log.trim().is_empty()
            && std::path::Path::new(&self.hooks.log).is_absolute()
            && !self.hooks.log.trim().starts_with('/')
        {
            return Err("agent.hooks.log is not a valid path".to_owned());
        }
        Ok(())
    }
}

fn env_parse<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::var(key)
        .ok()
        .and_then(|value| value.trim().parse().ok())
}

/// Install the process-wide default agent config (desktop settings loader).
///
/// Delegates to the contract registry (`zk_contract_agent`), so the desktop can
/// publish settings without depending on this crate. Invalid configs are
/// rejected before they are stored; the run loop snaps the config every turn.
///
/// # Errors
///
/// As [`AgentRuntimeConfig::validate`].
pub fn set_default_agent_config(config: AgentRuntimeConfig) -> Result<(), String> {
    config.validate()?;
    zk_contract_agent::set_default_agent_settings(&config.to_contract())?;
    let mut full = zk_contract_config::AppSettings {
        tools: config.tools,
        ..zk_contract_config::AppSettings::default()
    };
    full.agent = zk_contract_config::AgentSettings::default();
    zk_contract_config::set_runtime_settings(&full).map_err(|error| error.to_string())
}

/// The process-wide default agent config: the published contract settings when
/// present, else the `ZEOKEEL_AGENT_*` environment fallback.
#[must_use]
pub fn default_agent_config() -> AgentRuntimeConfig {
    let published_agent = zk_contract_agent::default_agent_settings();
    let published_tools = zk_contract_config::runtime_settings().ok().flatten();
    let mut config = published_agent
        .as_ref()
        .and_then(|settings| AgentRuntimeConfig::from_contract(settings).ok())
        .unwrap_or_else(AgentRuntimeConfig::from_env);
    if let Some(full) = published_tools {
        // Explicitly published tool settings win over env.
        config.tools = full.tools;
    } else if published_agent.is_some() {
        // Agent settings were saved but tool settings were not explicitly
        // published: keep the env/default fallback for timeouts.
        config.tools = AgentRuntimeConfig::from_env().tools;
    }
    config
}

impl ReasoningEffort {
    /// Contract string form.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::XHigh => "xhigh",
            Self::Max => "max",
        }
    }
}

impl PromptCache {
    /// Contract string form.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Short => "short",
            Self::Long => "long",
        }
    }
}

impl AgentRuntimeConfig {
    /// Convert this runtime config back into the contract `AgentSettings`
    /// (used when desktop hands it over through the contract registry).
    #[must_use]
    pub fn to_contract(&self) -> AgentSettings {
        AgentSettings {
            max_steps: self.max_steps,
            completion: zk_contract_config::CompletionSettings {
                temperature: self.completion.temperature,
                max_tokens: self.completion.max_tokens,
                top_p: self.completion.top_p,
                seed: self.completion.seed,
                stop: self.completion.stop.clone(),
                tool_choice: match &self.completion.tool_choice {
                    ToolChoiceConfig::Auto => ToolChoiceSetting::Auto,
                    ToolChoiceConfig::None => ToolChoiceSetting::None,
                    ToolChoiceConfig::Required => ToolChoiceSetting::Required,
                    ToolChoiceConfig::Specific(tools) => ToolChoiceSetting::Specific(tools.clone()),
                },
                parallel_tool_calls: self.completion.parallel_tool_calls,
                reasoning_effort: self
                    .completion
                    .reasoning_effort
                    .map(|effort| effort.as_str().to_owned()),
                prompt_cache: self
                    .completion
                    .prompt_cache
                    .map(|cache| cache.as_str().to_owned()),
            },
            memory: zk_contract_config::MemorySettings {
                enabled: self.memory.enabled,
                dir: self.memory.dir.clone(),
            },
            hooks: zk_contract_config::HookSettings {
                enabled: self.hooks.enabled,
                log: self.hooks.log.clone(),
            },
        }
    }

    /// Convert the contract `AgentSettings` (AppSettings.agent) into the
    /// runtime config, validating closed enums while mapping.
    ///
    /// # Errors
    ///
    /// A message naming the invalid field (reasoning effort / prompt cache are
    /// strings in the contract, enums in the runtime).
    pub fn from_contract(settings: &AgentSettings) -> Result<Self, String> {
        settings.validate().map_err(|error| error.to_string())?;
        let reasoning_effort = match settings.completion.reasoning_effort.as_deref() {
            None => None,
            Some(value) => Some(match value {
                "minimal" => ReasoningEffort::Minimal,
                "low" => ReasoningEffort::Low,
                "medium" => ReasoningEffort::Medium,
                "high" => ReasoningEffort::High,
                "xhigh" => ReasoningEffort::XHigh,
                "max" => ReasoningEffort::Max,
                other => {
                    return Err(format!(
                        "agent.completion.reasoning_effort must be minimal|low|medium|high|xhigh|max (got {other})"
                    ));
                }
            }),
        };
        let prompt_cache = match settings.completion.prompt_cache.as_deref() {
            None => None,
            Some("short") => Some(PromptCache::Short),
            Some("long") => Some(PromptCache::Long),
            Some(other) => {
                return Err(format!(
                    "agent.completion.prompt_cache must be short|long (got {other})"
                ));
            }
        };
        Ok(Self {
            completion: CompletionConfig {
                temperature: settings.completion.temperature,
                max_tokens: settings.completion.max_tokens,
                top_p: settings.completion.top_p,
                seed: settings.completion.seed,
                stop: settings.completion.stop.clone(),
                tool_choice: match &settings.completion.tool_choice {
                    ToolChoiceSetting::Auto => ToolChoiceConfig::Auto,
                    ToolChoiceSetting::None => ToolChoiceConfig::None,
                    ToolChoiceSetting::Required => ToolChoiceConfig::Required,
                    ToolChoiceSetting::Specific(tools) => ToolChoiceConfig::Specific(tools.clone()),
                },
                parallel_tool_calls: settings.completion.parallel_tool_calls,
                reasoning_effort,
                prompt_cache,
            },
            memory: MemoryConfig {
                enabled: settings.memory.enabled,
                dir: settings.memory.dir.clone(),
            },
            hooks: HooksConfig {
                enabled: settings.hooks.enabled,
                log: settings.hooks.log.clone(),
            },
            tools: AgentRuntimeConfig::from_env().tools,
            max_steps: settings.max_steps,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

    use super::*;

    #[test]
    fn env_fallback_and_published_settings_precedence() {
        // One test keeps the process-global registries deterministic: env
        // first, then the explicitly saved settings that desktop publishes.
        zk_contract_config::clear_runtime_settings().unwrap();
        std::env::set_var("ZEOKEEL_AGENT_TEMPERATURE", "0.42");
        std::env::set_var("ZEOKEEL_TOOL_TIMEOUT_MS", "5000");
        std::env::set_var("ZEOKEEL_APPROVAL_TIMEOUT_MS", "9000");

        // Fresh install: no published settings -> env fallbacks apply.
        let env_config = AgentRuntimeConfig::from_env();
        assert_eq!(env_config.completion.temperature, Some(0.42));
        assert_eq!(env_config.tools.tool_timeout_ms, 5000);
        assert_eq!(env_config.tools.approval_timeout_ms, 9000);
        assert_eq!(env_config.tools.tool_timeout(), 5000);

        // Explicitly saved settings (desktop publishes them) win over env.
        let saved = zk_contract_config::AgentSettings {
            completion: zk_contract_config::CompletionSettings {
                temperature: Some(0.7),
                ..zk_contract_config::CompletionSettings::default()
            },
            ..zk_contract_config::AgentSettings::default()
        };
        zk_contract_agent::set_default_agent_settings(&saved).unwrap();
        let mut full = zk_contract_config::AppSettings::default();
        full.tools.tool_timeout_ms = 2_000;
        full.tools.approval_timeout_ms = 4_000;
        zk_contract_config::set_runtime_settings(&full).unwrap();
        let effective = default_agent_config();
        assert_eq!(effective.completion.temperature, Some(0.7));
        assert_eq!(effective.tools.tool_timeout_ms, 2_000);
        assert_eq!(effective.tools.approval_timeout_ms, 4_000);

        // Out-of-range values are rejected at publish time (UI surfaces it);
        // the runtime clamp is covered by the contract tests.
        let mut invalid = full;
        invalid.tools.tool_timeout_ms = 10;
        assert!(zk_contract_config::set_runtime_settings(&invalid).is_err());
        invalid.tools.tool_timeout_ms = 5_000;
        invalid.tools.approval_timeout_ms = 9_000_000;
        assert!(zk_contract_config::set_runtime_settings(&invalid).is_err());

        // Restore so sibling tests keep seeing built-in defaults.
        zk_contract_config::clear_runtime_settings().unwrap();
        zk_contract_agent::set_default_agent_settings(&zk_contract_config::AgentSettings::default())
            .unwrap();
        std::env::remove_var("ZEOKEEL_AGENT_TEMPERATURE");
        std::env::remove_var("ZEOKEEL_TOOL_TIMEOUT_MS");
        std::env::remove_var("ZEOKEEL_APPROVAL_TIMEOUT_MS");
    }
}
