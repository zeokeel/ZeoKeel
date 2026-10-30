//! Enterprise gateway provider: only the login server, only Responses API.
#![forbid(unsafe_code)]

use serde_json::Value;
use zk_contract_llm::{LlmError, LlmProvider};

const DEFAULT_MODEL: &str = "deepseek-chat";

/// Gateway provider configured from bootstrap/env (never an upstream key).
pub struct GatewayLlm {
    base_url: String,
    access_token: String,
    model: String,
    http: reqwest::blocking::Client,
}

/// Factory used by generated glue (role `llm.provider.default`).
#[must_use]
pub fn provider() -> GatewayLlm {
    let base_url = std::env::var("ZEOKEEL_GATEWAY_BASE_URL").unwrap_or_default();
    let access_token = std::env::var("ZEOKEEL_GATEWAY_ACCESS_TOKEN").unwrap_or_default();
    let model = std::env::var("ZEOKEEL_GATEWAY_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_owned());
    GatewayLlm::new(base_url, access_token, model)
}

impl GatewayLlm {
    /// Explicit config (tests/bootstrap).
    #[must_use]
    pub fn new(base_url: String, access_token: String, model: String) -> Self {
        Self { base_url: base_url.trim_end_matches('/').to_owned(), access_token, model, http: reqwest::blocking::Client::new() }
    }

    fn text_from(value: &Value) -> Option<String> {
        let mut text = String::new();
        if let Some(output) = value.get("output").and_then(Value::as_array) {
            for item in output {
                if let Some(content) = item.get("content").and_then(Value::as_array) {
                    for part in content {
                        if let Some(part_text) = part.get("text").and_then(Value::as_str) {
                            text.push_str(part_text);
                        }
                    }
                }
                if let Some(item_text) = item.get("text").and_then(Value::as_str) {
                    text.push_str(item_text);
                }
            }
        }
        if text.is_empty() {
            value.get("output_text").and_then(Value::as_str).map(str::to_owned)
        } else {
            Some(text)
        }
    }
}

impl LlmProvider for GatewayLlm {
    fn complete(&self, prompt: &str) -> Result<String, LlmError> {
        if self.base_url.is_empty() {
            return Err(LlmError::InvalidRequest("ZEOKEEL_GATEWAY_BASE_URL is empty".to_owned()));
        }
        if self.access_token.is_empty() {
            return Err(LlmError::InvalidRequest("gateway access token is missing".to_owned()));
        }
        let response = self
            .http
            .post(format!("{}/v1/responses", self.base_url))
            .bearer_auth(&self.access_token)
            .json(&serde_json::json!({ "model": self.model, "input": prompt }))
            .send()
            .map_err(|error| LlmError::Upstream(error.to_string()))?;
        let status = response.status();
        let body = response.text().map_err(|error| LlmError::Upstream(error.to_string()))?;
        if !status.is_success() {
            return Err(LlmError::Upstream(format!("gateway HTTP {}", status.as_u16())));
        }
        let value: Value = serde_json::from_str(&body).map_err(|_| LlmError::Upstream("invalid gateway response".to_owned()))?;
        Self::text_from(&value).ok_or_else(|| LlmError::Upstream("gateway response has no output text".to_owned()))
    }
}
