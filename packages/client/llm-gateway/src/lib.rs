//! Enterprise gateway provider: only the login server, only Responses API.
#![forbid(unsafe_code)]

use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::sync::{OnceLock, RwLock};
use zk_contract_llm::{LlmError, LlmProvider};

const DEFAULT_MODEL: &str = "deepseek-chat";

#[derive(Clone)]
struct GatewayConfig {
    base_url: String,
    access_token: String,
    model: String,
}

impl GatewayConfig {
    fn from_env() -> Self {
        Self {
            base_url: std::env::var("ZEOKEEL_GATEWAY_BASE_URL").unwrap_or_default(),
            access_token: std::env::var("ZEOKEEL_GATEWAY_ACCESS_TOKEN").unwrap_or_default(),
            model: std::env::var("ZEOKEEL_GATEWAY_MODEL")
                .unwrap_or_else(|_| DEFAULT_MODEL.to_owned()),
        }
    }
}

fn config() -> &'static RwLock<GatewayConfig> {
    static CONFIG: OnceLock<RwLock<GatewayConfig>> = OnceLock::new();
    CONFIG.get_or_init(|| RwLock::new(GatewayConfig::from_env()))
}

/// Inject the login server URL and access token after client login.
pub fn set_credentials(base_url: &str, access_token: &str) {
    if let Ok(mut config) = config().write() {
        config.base_url = base_url.trim_end_matches('/').to_owned();
        config.access_token = access_token.to_owned();
    }
}

/// Current injected credentials (base URL, access token), if configured.
#[must_use]
pub fn credentials() -> Option<(String, String)> {
    let config = config().read().ok()?;
    if config.base_url.is_empty() || config.access_token.is_empty() {
        None
    } else {
        Some((config.base_url.clone(), config.access_token.clone()))
    }
}

/// Override the default gateway model (from bootstrap catalog).
pub fn set_model(model: &str) {
    if model.trim().is_empty() {
        return;
    }
    if let Ok(mut config) = config().write() {
        config.model = model.to_owned();
    }
}

/// Gateway provider configured from env or the runtime credential injection.
pub struct GatewayLlm {
    http: reqwest::blocking::Client,
}

/// Factory used by generated glue (role `llm.provider.default`).
#[must_use]
pub fn provider() -> GatewayLlm {
    GatewayLlm {
        http: reqwest::blocking::Client::new(),
    }
}

impl GatewayLlm {
    /// Explicit config (tests/bootstrap); also updates the shared credential slot.
    #[must_use]
    pub fn new(base_url: String, access_token: String, model: String) -> Self {
        if let Ok(mut config) = config().write() {
            config.base_url = base_url.trim_end_matches('/').to_owned();
            config.access_token = access_token;
            config.model = model;
        }
        Self {
            http: reqwest::blocking::Client::new(),
        }
    }

    fn credentials(&self) -> Result<(String, String, String), LlmError> {
        match config().read() {
            Ok(config) => {
                if config.base_url.is_empty() {
                    return Err(LlmError::InvalidRequest(
                        "gateway server URL is missing".to_owned(),
                    ));
                }
                if config.access_token.is_empty() {
                    return Err(LlmError::InvalidRequest(
                        "gateway access token is missing".to_owned(),
                    ));
                }
                Ok((
                    config.base_url.clone(),
                    config.access_token.clone(),
                    config.model.clone(),
                ))
            }
            Err(_) => Err(LlmError::InvalidRequest(
                "gateway config poisoned".to_owned(),
            )),
        }
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
            value
                .get("output_text")
                .and_then(Value::as_str)
                .map(str::to_owned)
        } else {
            Some(text)
        }
    }
}

impl LlmProvider for GatewayLlm {
    fn complete_stream(
        &self,
        prompt: &str,
        on_delta: &mut dyn FnMut(&str),
    ) -> Result<String, LlmError> {
        let (base_url, access_token, model) = self.credentials()?;
        let response = self
            .http
            .post(format!("{base_url}/v1/responses"))
            .bearer_auth(&access_token)
            .json(&serde_json::json!({ "model": model, "input": prompt, "stream": true }))
            .send()
            .map_err(|error| LlmError::Upstream(error.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            return Err(LlmError::Upstream(format!(
                "gateway HTTP {}",
                status.as_u16()
            )));
        }
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");
        if !content_type.starts_with("text/event-stream") {
            let body = response
                .text()
                .map_err(|error| LlmError::Upstream(error.to_string()))?;
            let value: Value = serde_json::from_str(&body)
                .map_err(|_| LlmError::Upstream("invalid gateway response".to_owned()))?;
            let text = Self::text_from(&value).ok_or_else(|| {
                LlmError::Upstream("gateway response has no output text".to_owned())
            })?;
            on_delta(&text);
            return Ok(text);
        }
        let mut full = String::new();
        for line in BufReader::new(response).lines() {
            let line = line.map_err(|error| LlmError::Upstream(error.to_string()))?;
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data == "[DONE]" {
                break;
            }
            let Ok(value) = serde_json::from_str::<Value>(data) else {
                continue;
            };
            if let Some(delta) = value.get("delta").and_then(Value::as_str) {
                full.push_str(delta);
                on_delta(delta);
            }
        }
        Ok(full)
    }

    fn complete(&self, prompt: &str) -> Result<String, LlmError> {
        let (base_url, access_token, model) = self.credentials()?;
        let response = self
            .http
            .post(format!("{base_url}/v1/responses"))
            .bearer_auth(&access_token)
            .json(&serde_json::json!({ "model": model, "input": prompt }))
            .send()
            .map_err(|error| LlmError::Upstream(error.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .map_err(|error| LlmError::Upstream(error.to_string()))?;
        if !status.is_success() {
            return Err(LlmError::Upstream(format!(
                "gateway HTTP {}",
                status.as_u16()
            )));
        }
        let value: Value = serde_json::from_str(&body)
            .map_err(|_| LlmError::Upstream("invalid gateway response".to_owned()))?;
        Self::text_from(&value)
            .ok_or_else(|| LlmError::Upstream("gateway response has no output text".to_owned()))
    }
}
