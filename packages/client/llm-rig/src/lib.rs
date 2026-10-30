//! rig model provider for personal edition.
//!
//! @capability llm.provider.rig
#![forbid(unsafe_code)]

use rig_core::operation::Completion;
use rig_core::providers::openai::OpenAIConfig;
use rig_core::DynModel;
use zk_contract_llm::{LlmError, LlmProvider};

/// Provider configuration resolved from environment/settings.
#[derive(Debug, Clone, Default)]
pub struct RigConfig {
    /// Provider: openai | deepseek | anthropic | ollama.
    pub provider: String,
    /// OpenAI-compatible endpoint (empty => provider default).
    pub base_url: String,
    /// Bearer key.
    pub api_key: String,
    /// Model id.
    pub model: String,
}

impl RigConfig {
    /// Resolve from `ZEOKEEL_*` environment (live-dev fallback).
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            provider: {
                let provider = env_first(&["ZEOKEEL_RIG_PROVIDER"]);
                if provider.is_empty() {
                    "openai".to_owned()
                } else {
                    provider
                }
            },
            base_url: env_first(&["ZEOKEEL_OPENAI_BASE_URL", "ZEOKEEL_DEV_OPENAI_BASE_URL"]),
            api_key: env_first(&["ZEOKEEL_OPENAI_API_KEY", "ZEOKEEL_DEV_OPENAI_API_KEY"]),
            model: {
                let model = env_first(&["ZEOKEEL_OPENAI_MODEL", "ZEOKEEL_DEV_MODEL"]);
                if model.is_empty() {
                    "gpt-4o-mini".to_owned()
                } else {
                    model
                }
            },
        }
    }
}

fn env_first(keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| {
            std::env::var(key)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_default()
}

/// Build a model from explicit fields (settings hot-swap).
pub fn model_from_config(
    provider: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<DynModel<Completion>, LlmError> {
    model_from(&RigConfig {
        provider: provider.to_owned(),
        base_url: base_url.to_owned(),
        api_key: api_key.to_owned(),
        model: model.to_owned(),
    })
}

/// Build the rig completion model.
pub fn model_from(config: &RigConfig) -> Result<DynModel<Completion>, LlmError> {
    if config.base_url.trim().is_empty() && config.api_key.trim().is_empty() {
        return Err(LlmError::InvalidRequest(
            "rig provider not configured (base_url/api_key)".to_owned(),
        ));
    }
    if config.model.trim().is_empty() {
        return Err(LlmError::InvalidRequest("rig model is empty".to_owned()));
    }
    match config.provider.as_str() {
        "openai" => {
            let openai = OpenAIConfig::new(config.api_key.clone())
                .with_base_url(config.base_url.trim_end_matches('/').to_owned())
                .client();
            Ok(openai.chat(config.model.clone()).into())
        }
        "deepseek" => {
            let base = if config.base_url.trim().is_empty() {
                "https://api.deepseek.com".to_owned()
            } else {
                config.base_url.trim_end_matches('/').to_owned()
            };
            let client = OpenAIConfig::with_key(
                &rig_core::providers::openai::wire::DEEPSEEK,
                config.api_key.clone(),
            )
            .with_base_url(base)
            .client();
            Ok(client.chat(config.model.clone()).into())
        }
        "anthropic" => {
            if config.api_key.trim().is_empty() {
                return Err(LlmError::InvalidRequest(
                    "anthropic api_key is empty".to_owned(),
                ));
            }
            let mut anthropic = rig_core::providers::anthropic::AnthropicConfig::with_key(
                &rig_core::providers::anthropic::ANTHROPIC,
                config.api_key.clone(),
            );
            if !config.base_url.trim().is_empty() {
                anthropic = anthropic.with_base_url(config.base_url.trim_end_matches('/'));
            }
            Ok(anthropic.client().completion(config.model.clone()).into())
        }
        "ollama" => {
            let base = if config.base_url.trim().is_empty() {
                "http://127.0.0.1:11434".to_owned()
            } else {
                config.base_url.trim_end_matches('/').to_owned()
            };
            let client = rig_core::providers::ollama::OllamaConfig::new()
                .with_base_url(base)
                .with_api_key(config.api_key.clone())
                .client();
            Ok(client.completion(config.model.clone()).into())
        }
        other => Err(LlmError::InvalidRequest(format!(
            "unknown rig provider: {other}"
        ))),
    }
}

/// Default factory used by generated glue.
pub fn model() -> Result<DynModel<Completion>, LlmError> {
    model_from(&RigConfig::from_env())
}

/// Legacy provider adapter kept for package contract compatibility.
#[derive(Debug, Clone, Default)]
pub struct RigProvider;

impl LlmProvider for RigProvider {
    fn complete(&self, _prompt: &str) -> Result<String, LlmError> {
        Err(LlmError::InvalidRequest(
            "use rig model(), not legacy complete()".to_owned(),
        ))
    }
}

/// Legacy factory.
#[must_use]
pub fn provider() -> RigProvider {
    RigProvider
}
