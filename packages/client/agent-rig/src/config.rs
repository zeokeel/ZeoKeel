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

/// Default model context window when no override is configured (docs/38 F4).
pub const DEFAULT_CONTEXT_WINDOW_TOKENS: u64 = 128_000;
/// Tokens reserved for the completion (output) inside the context window.
pub const CONTEXT_OUTPUT_RESERVE_TOKENS: u64 = 4_096;

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

/// Optional JSONL effect log (B9 subset: rig effect records, not live checkpoint).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct EffectsConfig {
    /// Whether rig effect records are appended to `path`.
    pub enabled: bool,
    /// Append-only JSONL file. Empty means disabled.
    pub path: String,
}

/// Context-window budget (B10 subset: budget + deterministic truncation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ContextConfig {
    /// Prompt budget in tokens. `None` uses the built-in 128k default (docs/38 F4).
    pub window_tokens: Option<u64>,
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
    /// Optional JSONL effect log (rig `Recorder`; B9 subset).
    pub effects: EffectsConfig,
    /// Context-window budget + deterministic truncation (B10 subset).
    pub context: ContextConfig,
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
            effects: EffectsConfig::default(),
            context: ContextConfig::default(),
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
        if let Ok(value) = std::env::var("ZEOKEEL_AGENT_EFFECT_LOG_ENABLED") {
            config.effects.enabled = value != "0";
        }
        if let Ok(value) = std::env::var("ZEOKEEL_AGENT_EFFECT_LOG") {
            config.effects.path = value;
            config.effects.enabled = true;
        }
        if let Ok(value) = std::env::var("ZEOKEEL_AGENT_CONTEXT_WINDOW_TOKENS") {
            if let Ok(tokens) = value.parse::<u64>() {
                config.context.window_tokens = Some(tokens);
            }
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

    /// Resolve the effect-log path, if effects logging is enabled and configured.
    #[must_use]
    pub fn effect_log_path(&self) -> Option<std::path::PathBuf> {
        if !self.effects.enabled || self.effects.path.trim().is_empty() {
            return None;
        }
        Some(std::path::PathBuf::from(self.effects.path.trim()))
    }

    /// Prompt budget in tokens: explicit override or the 128k default (docs/38 F4).
    #[must_use]
    pub fn prompt_budget_tokens(&self) -> u64 {
        self.context
            .window_tokens
            .unwrap_or(DEFAULT_CONTEXT_WINDOW_TOKENS)
            .saturating_sub(CONTEXT_OUTPUT_RESERVE_TOKENS)
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
    // Publish one non-secret snapshot containing both sections (agent + tools)
    // so a later publish can never overwrite the other section with defaults.
    // `web_search` is the desktop's/tool's section: preserve whatever is
    // already published instead of clearing it with this agent-only publish.
    let agent = config.to_contract();
    zk_contract_agent::set_default_agent_settings(&agent)?;
    let web_search = zk_contract_config::runtime_settings()
        .map_err(|error| error.to_string())?
        .and_then(|snapshot| snapshot.web_search);
    zk_contract_config::set_runtime_settings(&zk_contract_config::RuntimeSettings {
        agent: Some(agent),
        tools: Some(config.tools),
        web_search,
    })
    .map_err(|error| error.to_string())
}

/// Whether the `ZEOKEEL_AGENT_*` environment explicitly changed an agent
/// setting (tools are handled separately).
fn env_agent_is_overridden(env: &AgentRuntimeConfig) -> bool {
    let defaults = AgentRuntimeConfig::default();
    env.completion != defaults.completion
        || env.memory != defaults.memory
        || env.hooks != defaults.hooks
        || env.max_steps != defaults.max_steps
}

/// The process-wide default agent config, merged per section:
/// published snapshot section > explicit `ZEOKEEL_*` env > legacy
/// contract-agent registry > built-in defaults.
///
/// The legacy registry is only a fallback for hosts that never publish a
/// runtime snapshot (and only when the env has no explicit agent override): a
/// tools-only save must not pin agent defaults over the environment.
#[must_use]
pub fn default_agent_config() -> AgentRuntimeConfig {
    let published = zk_contract_config::runtime_settings().ok().flatten();
    let snapshot_agent = published
        .as_ref()
        .and_then(|snapshot| snapshot.agent.clone());
    let env = AgentRuntimeConfig::from_env();
    let mut config = if let Some(agent) = snapshot_agent {
        AgentRuntimeConfig::from_contract(&agent).unwrap_or(env)
    } else if env_agent_is_overridden(&env) {
        env
    } else if let Some(legacy) = zk_contract_agent::default_agent_settings() {
        AgentRuntimeConfig::from_contract(&legacy).unwrap_or(env)
    } else {
        env
    };
    config.tools = published
        .and_then(|snapshot| snapshot.tools)
        .unwrap_or_else(|| AgentRuntimeConfig::from_env().tools);
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
            // Schedule auto-run is desktop-owned (AppSettings.agent.schedule);
            // the agent runtime only carries it through unchanged snapshots.
            schedule: zk_contract_config::ScheduleSettings::default(),
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
            // The contract `AgentSettings` has no tool/effect-log section, so seed
            // env/default fallbacks; explicit per-instance config overrides.
            effects: AgentRuntimeConfig::from_env().effects,
            context: AgentRuntimeConfig::from_env().context,
            tools: AgentRuntimeConfig::from_env().tools,
            max_steps: settings.max_steps,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

    use super::*;

    fn snapshot(
        agent: Option<zk_contract_config::AgentSettings>,
        tools: Option<zk_contract_config::ToolSettings>,
    ) -> zk_contract_config::RuntimeSettings {
        zk_contract_config::RuntimeSettings {
            agent,
            tools,
            web_search: None,
        }
    }

    fn agent_settings(max_steps: usize, temperature: f64) -> zk_contract_config::AgentSettings {
        zk_contract_config::AgentSettings {
            max_steps,
            completion: zk_contract_config::CompletionSettings {
                temperature: Some(temperature),
                ..zk_contract_config::CompletionSettings::default()
            },
            ..zk_contract_config::AgentSettings::default()
        }
    }

    fn tool_settings(
        tool_timeout_ms: u64,
        approval_timeout_ms: u64,
    ) -> zk_contract_config::ToolSettings {
        zk_contract_config::ToolSettings {
            tool_timeout_ms,
            approval_timeout_ms,
        }
    }

    #[test]
    fn per_section_precedence_between_published_settings_and_env() {
        // One ordered test keeps the process-global registries deterministic.
        zk_contract_config::clear_runtime_settings().unwrap();
        std::env::set_var("ZEOKEEL_AGENT_TEMPERATURE", "0.42");
        std::env::set_var("ZEOKEEL_AGENT_MAX_STEPS", "33");
        std::env::set_var("ZEOKEEL_TOOL_TIMEOUT_MS", "5000");
        std::env::set_var("ZEOKEEL_APPROVAL_TIMEOUT_MS", "9000");

        // Fresh install: no published sections -> env fallbacks apply.
        let env_config = default_agent_config();
        assert_eq!(env_config.completion.temperature, Some(0.42));
        assert_eq!(env_config.max_steps, 33);
        assert_eq!(env_config.tools.tool_timeout_ms, 5_000);
        assert_eq!(env_config.tools.approval_timeout_ms, 9_000);

        // Legacy host published an agent registry only: explicit env still wins.
        zk_contract_agent::set_default_agent_settings(&agent_settings(12, 0.7)).unwrap();
        let config = default_agent_config();
        assert_eq!(config.max_steps, 33, "env agent wins over legacy registry");
        assert_eq!(config.completion.temperature, Some(0.42));

        // Without an explicit env agent override the legacy registry applies.
        std::env::remove_var("ZEOKEEL_AGENT_TEMPERATURE");
        std::env::remove_var("ZEOKEEL_AGENT_MAX_STEPS");
        let config = default_agent_config();
        assert_eq!(config.max_steps, 12);
        assert_eq!(config.completion.temperature, Some(0.7));

        // Restore env so the tools-only scenario exercises env > legacy again.
        std::env::set_var("ZEOKEEL_AGENT_TEMPERATURE", "0.42");
        std::env::set_var("ZEOKEEL_AGENT_MAX_STEPS", "33");

        // Tools-only saved: published tools win, agent keeps env fallback over
        // the legacy registry (the round-4 P1 regression).
        zk_contract_config::set_runtime_settings(&snapshot(
            None,
            Some(tool_settings(2_000, 4_000)),
        ))
        .unwrap();
        let config = default_agent_config();
        assert_eq!(
            config.max_steps, 33,
            "tools-only save must not pin agent defaults"
        );
        assert_eq!(config.completion.temperature, Some(0.42));
        assert_eq!(config.tools.tool_timeout_ms, 2_000);
        assert_eq!(config.tools.approval_timeout_ms, 4_000);

        // Agent-only saved: published agent wins, tools keep env fallback.
        zk_contract_config::set_runtime_settings(&snapshot(Some(agent_settings(12, 0.7)), None))
            .unwrap();
        let config = default_agent_config();
        assert_eq!(config.max_steps, 12);
        assert_eq!(config.completion.temperature, Some(0.7));
        assert_eq!(
            config.tools.tool_timeout_ms, 5_000,
            "agent-only save must not pin tool defaults"
        );
        assert_eq!(config.tools.approval_timeout_ms, 9_000);

        // Both sections saved: saved values win for both.
        zk_contract_config::set_runtime_settings(&snapshot(
            Some(agent_settings(12, 0.7)),
            Some(tool_settings(3_000, 6_000)),
        ))
        .unwrap();
        let config = default_agent_config();
        assert_eq!(config.max_steps, 12);
        assert_eq!(config.completion.temperature, Some(0.7));
        assert_eq!(config.tools.tool_timeout_ms, 3_000);
        assert_eq!(config.tools.approval_timeout_ms, 6_000);

        // Programmatic entry point publishes both sections in one snapshot.
        let programmatic = AgentRuntimeConfig {
            max_steps: 5,
            completion: CompletionConfig {
                temperature: Some(0.55),
                ..CompletionConfig::default()
            },
            tools: tool_settings(1_500, 2_500),
            ..AgentRuntimeConfig::default()
        };
        set_default_agent_config(programmatic).unwrap();
        let published = zk_contract_config::runtime_settings().unwrap().unwrap();
        assert_eq!(published.agent.as_ref().unwrap().max_steps, 5);
        assert_eq!(published.tools.unwrap().tool_timeout_ms, 1_500);
        let config = default_agent_config();
        assert_eq!(config.max_steps, 5);
        assert_eq!(config.completion.temperature, Some(0.55));
        assert_eq!(config.tools.tool_timeout_ms, 1_500);

        // Invalid values inside a present section are rejected at publish time.
        assert!(zk_contract_config::set_runtime_settings(&snapshot(
            None,
            Some(tool_settings(10, 300_000)),
        ))
        .is_err());

        // Restore so sibling tests keep seeing built-in defaults.
        zk_contract_config::clear_runtime_settings().unwrap();
        zk_contract_agent::clear_default_agent_settings().unwrap();
        std::env::remove_var("ZEOKEEL_AGENT_TEMPERATURE");
        std::env::remove_var("ZEOKEEL_AGENT_MAX_STEPS");
        std::env::remove_var("ZEOKEEL_TOOL_TIMEOUT_MS");
        std::env::remove_var("ZEOKEEL_APPROVAL_TIMEOUT_MS");
    }
}
