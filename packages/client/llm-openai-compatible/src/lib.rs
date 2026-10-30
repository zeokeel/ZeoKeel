//! OpenAI-compatible provider (personal edition, Chat Completions).
//!
//! @capability llm.provider.openai_compatible
#![forbid(unsafe_code)]

use std::time::Duration;

use serde::Deserialize;
use zk_contract_llm::{LlmError, LlmProvider};

const TIMEOUT: Duration = Duration::from_secs(120);

/// OpenAI-compatible provider configured from local settings/env.
pub struct OpenAiCompatible {
    base_url: String,
    api_key: String,
    model: String,
    client: reqwest::blocking::Client,
}

/// Default factory used by generated glue (role `llm.provider.default`).
#[must_use]
pub fn provider() -> OpenAiCompatible {
    let base_url = env_first(&["ZEOKEEL_OPENAI_BASE_URL", "ZEOKEEL_DEV_OPENAI_BASE_URL"]);
    let api_key = env_first(&["ZEOKEEL_OPENAI_API_KEY", "ZEOKEEL_DEV_OPENAI_API_KEY"]);
    let model = {
        let m = env_first(&["ZEOKEEL_OPENAI_MODEL", "ZEOKEEL_DEV_MODEL"]);
        if m.is_empty() {
            "gpt-4o-mini".to_owned()
        } else {
            m
        }
    };
    OpenAiCompatible::new(base_url, api_key, model)
}

fn env_first(keys: &[&str]) -> String {
    for key in keys {
        if let Ok(value) = std::env::var(key) {
            if !value.trim().is_empty() {
                return value;
            }
        }
    }
    String::new()
}

impl OpenAiCompatible {
    /// Create a provider with explicit config (preferred for tests/settings).
    #[must_use]
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            api_key,
            model,
            client,
        }
    }
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}
#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}
#[derive(Deserialize)]
struct ChatMessage {
    #[serde(default)]
    content: String,
}

impl LlmProvider for OpenAiCompatible {
    fn complete(&self, prompt: &str) -> Result<String, LlmError> {
        if self.base_url.is_empty() {
            return Err(LlmError::InvalidRequest(
                "base_url is empty (set ZEOKEEL_OPENAI_BASE_URL / ZEOKEEL_DEV_OPENAI_BASE_URL)"
                    .to_owned(),
            ));
        }
        if self.model.trim().is_empty() {
            return Err(LlmError::InvalidRequest("model is empty".to_owned()));
        }
        let url = format!("{}/chat/completions", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "messages": [{ "role": "user", "content": prompt }],
            "temperature": 0.2,
            "max_tokens": 2048,
            "stream": false,
        });
        let mut request = self.client.post(&url).json(&body);
        if !self.api_key.is_empty() {
            request = request.bearer_auth(&self.api_key);
        }
        let response = request
            .send()
            .map_err(|error| LlmError::Upstream(error.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().unwrap_or_default();
            return Err(LlmError::Upstream(format!(
                "HTTP {status}: {}",
                truncate(&text, 400)
            )));
        }
        let parsed: ChatResponse = response
            .json()
            .map_err(|error| LlmError::Upstream(error.to_string()))?;
        parsed
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .filter(|content| !content.trim().is_empty())
            .ok_or_else(|| LlmError::Upstream("empty completion".to_owned()))
    }
}

fn truncate(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}
