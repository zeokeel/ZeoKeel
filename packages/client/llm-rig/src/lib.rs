//! rig model provider for personal edition.
//!
//! @capability llm.provider.rig
#![forbid(unsafe_code)]

use rig_core::providers::openai::{OpenAIConfig, OpenAI};
use rig_core::operation::Completion;
use rig_core::DynModel;
use zk_contract_llm::{LlmError, LlmProvider};

/// Provider configuration resolved from environment/settings.
#[derive(Debug, Clone, Default)]
pub struct RigConfig {
    /// OpenAI-compatible endpoint (empty => OpenAI default).
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
            base_url: env_first(&["ZEOKEEL_OPENAI_BASE_URL", "ZEOKEEL_DEV_OPENAI_BASE_URL"]),
            api_key: env_first(&["ZEOKEEL_OPENAI_API_KEY", "ZEOKEEL_DEV_OPENAI_API_KEY"]),
            model: {
                let model = env_first(&["ZEOKEEL_OPENAI_MODEL", "ZEOKEEL_DEV_MODEL"]);
                if model.is_empty() { "gpt-4o-mini".to_owned() } else { model }
            },
        }
    }
}

fn env_first(keys: &[&str]) -> String {
    keys.iter().find_map(|key| std::env::var(key).ok().filter(|value| !value.trim().is_empty())).unwrap_or_default()
}

/// Build the rig completion model.
pub fn model_from(config: &RigConfig) -> Result<DynModel<Completion>, LlmError> {
    if config.base_url.trim().is_empty() && config.api_key.trim().is_empty() {
        return Err(LlmError::InvalidRequest("rig provider not configured (base_url/api_key)".to_owned()));
    }
    if config.model.trim().is_empty() {
        return Err(LlmError::InvalidRequest("rig model is empty".to_owned()));
    }
    let mut openai = OpenAIConfig::new(config.api_key.clone()).with_base_url(config.base_url.trim_end_matches('/').to_owned()).client();
    let model: DynModel<Completion> = openai.chat(config.model.clone()).into();
    let _: &mut OpenAI = &mut openai;
    Ok(model)
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
        Err(LlmError::InvalidRequest("use rig model(), not legacy complete()".to_owned()))
    }
}

/// Legacy factory.
#[must_use]
pub fn provider() -> RigProvider { RigProvider }
