//! Enterprise gateway provider: only the login server, only Responses API.
#![forbid(unsafe_code)]

use rig_core::operation::Completion;
use rig_core::providers::openai::OpenAIConfig;
use rig_core::DynModel;
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::sync::{OnceLock, RwLock};
use zk_contract_llm::{LlmError, LlmProvider};

const DEFAULT_MODEL: &str = "deepseek-chat";

/// Closed local port the pre-login placeholder model points at (ADR-064 ①):
/// a turn started before login fails loudly at the transport layer instead of
/// silently using a mock model.
const UNCONFIGURED_BASE_URL: &str = "http://127.0.0.1:9";
/// Model id reported by the pre-login placeholder (never sent: no request can
/// reach the closed port).
const UNCONFIGURED_MODEL: &str = "unconfigured";
/// Bearer value of the pre-login placeholder; never leaves the process.
const UNCONFIGURED_TOKEN: &str = "unconfigured";

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

/// Build the rig completion model that sends through the login server's
/// enterprise gateway.
///
/// The gateway only speaks the OpenAI Responses API (`POST /v1/responses`).
/// rig's native Responses wire is bound to `{server}/v1`, so the request path
/// resolves to `{server}/v1/responses`, and the login access token is sent as
/// `Authorization: Bearer <token>`. Credentials and the model come from
/// [`set_credentials`] / [`set_model`] (login + bootstrap catalog), so the
/// model must be rebuilt after login or after the catalog changes (ADR-064).
///
/// When the shared credentials or the bootstrap model are absent (the normal
/// state before the enterprise login page) this returns
/// [`unconfigured_model`] so `zk_composed::build()` can boot and render the
/// login UI instead of exiting before login. Any turn started in that state
/// fails at the transport layer (no request can reach the closed local port);
/// after login the generated glue's `model_from_config` factory builds the
/// real model through the desktop `Runtime::configure_llm` hot-reload path.
///
/// # Errors
///
/// Returns [`LlmError::InvalidRequest`] when the config lock is poisoned or a
/// fully supplied server URL is not `http(s)://`; it does not return a model
/// that would silently call a mock provider.
pub fn model() -> Result<DynModel<Completion>, LlmError> {
    let config = config().read().map_err(|_| {
        LlmError::InvalidRequest(
            "gateway config poisoned; could not read login credentials".to_owned(),
        )
    })?;
    if config.base_url.trim().is_empty()
        || config.access_token.trim().is_empty()
        || config.model.trim().is_empty()
    {
        return unconfigured_model();
    }
    model_from_parts(&config.base_url, &config.access_token, &config.model)
}

/// Build the pre-login placeholder model (ADR-064 ①).
///
/// It is a real Responses model pointed at a closed local port
/// (`127.0.0.1:9`), so the composed desktop can boot before login while any
/// actual call fails at the transport layer instead of silently answering as
/// a mock. Mirrors `zk_llm_rig::unconfigured_model`.
///
/// # Errors
///
/// Construction cannot fail for the fixed placeholder values; the `Result`
/// keeps the same shape as [`model`].
pub fn unconfigured_model() -> Result<DynModel<Completion>, LlmError> {
    model_from_parts(
        UNCONFIGURED_BASE_URL,
        UNCONFIGURED_TOKEN,
        UNCONFIGURED_MODEL,
    )
}

/// Build the rig completion model from explicit values, as generated glue does
/// for settings hot-reload.
///
/// Empty `base_url`/`api_key`/`model` fall back to the login-injected shared
/// config ([`set_credentials`] / [`set_model`]). The only accepted provider is
/// the gateway itself (aliases `gateway`, `openai`, `openai_responses`,
/// `responses`, or empty); local provider kinds such as `mock`/`ollama` are
/// rejected because enterprise AI must go through the login server (ADR-031).
///
/// # Errors
///
/// [`LlmError::InvalidRequest`] for an unsupported provider, a partially
/// supplied config (one of server URL/access token missing while the other is
/// set), an empty model when credentials are present, or a URL that is not
/// `http(s)://`.
///
/// With no login credentials at all (pre-login boot and the desktop logout
/// reset, `configure_llm("", "", "", "gateway")`) it returns
/// [`unconfigured_model`], so the agent keeps a model whose calls fail at the
/// transport layer until the next login replaces it.
pub fn model_from_config(
    provider: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<DynModel<Completion>, LlmError> {
    let provider = provider.trim();
    if !matches!(
        provider,
        "" | "gateway" | "openai" | "openai_responses" | "responses"
    ) {
        return Err(LlmError::InvalidRequest(format!(
            "gateway does not support provider `{provider}`; enterprise AI is Responses-only through the login server"
        )));
    }
    let (shared_base, shared_token, shared_model) = {
        let config = config().read().map_err(|_| {
            LlmError::InvalidRequest(
                "gateway config poisoned; could not read login credentials".to_owned(),
            )
        })?;
        (
            config.base_url.clone(),
            config.access_token.clone(),
            config.model.clone(),
        )
    };
    let base = if base_url.trim().is_empty() {
        shared_base
    } else {
        base_url.trim_end_matches('/').to_owned()
    };
    let token = if api_key.trim().is_empty() {
        shared_token
    } else {
        api_key.to_owned()
    };
    let model = if model.trim().is_empty() {
        shared_model
    } else {
        model.to_owned()
    };
    // Logout / pre-login: no login credentials at all -> boot placeholder so the
    // agent keeps running (calls fail at the transport layer until the next
    // login hot-swaps the real model). A partially supplied config is still an
    // error so broken login/bootstrap surfaces instead of a placeholder.
    if base.trim().is_empty() && token.trim().is_empty() {
        return unconfigured_model();
    }
    model_from_parts(&base, &token, &model)
}

/// Validate explicit parts and build the rig Responses model, or explain
/// exactly which login/bootstrap step is missing.
fn model_from_parts(
    base_url: &str,
    access_token: &str,
    model: &str,
) -> Result<DynModel<Completion>, LlmError> {
    let server = base_url.trim().trim_end_matches('/');
    if server.is_empty() {
        return Err(LlmError::InvalidRequest(
            "gateway login required: server URL is missing; complete login before using the AI gateway"
                .to_owned(),
        ));
    }
    if !(server.starts_with("http://") || server.starts_with("https://")) {
        return Err(LlmError::InvalidRequest(format!(
            "gateway server URL is invalid: `{server}`"
        )));
    }
    let token = access_token.trim();
    if token.is_empty() {
        return Err(LlmError::InvalidRequest(
            "gateway login required: access token is missing; complete login before using the AI gateway"
                .to_owned(),
        ));
    }
    let model = model.trim();
    if model.is_empty() {
        return Err(LlmError::InvalidRequest(
            "gateway model is missing; wait for the bootstrap model catalog (or set_model) before using the AI gateway"
                .to_owned(),
        ));
    }
    let config = OpenAIConfig::new(token.to_owned()).with_base_url(gateway_v1_base(server));
    Ok(config.client().responses(model.to_owned()).into())
}

/// The rig base URL for the gateway: `{server}/v1`, unless the injected server
/// URL already names the version segment.
fn gateway_v1_base(server: &str) -> String {
    if server.ends_with("/v1") {
        server.to_owned()
    } else {
        format!("{server}/v1")
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

    use super::*;

    #[test]
    fn gateway_v1_base_appends_version_once() {
        assert_eq!(
            gateway_v1_base("http://127.0.0.1:8795"),
            "http://127.0.0.1:8795/v1"
        );
        assert_eq!(
            gateway_v1_base("https://srv.example/v1"),
            "https://srv.example/v1"
        );
    }

    #[test]
    fn model_parts_require_login_and_model() {
        let missing_server = model_from_parts("", "token", "model").unwrap_err();
        assert!(
            missing_server.to_string().contains("login required"),
            "{missing_server}"
        );
        let missing_token = model_from_parts("http://127.0.0.1:1", "  ", "model").unwrap_err();
        assert!(
            missing_token.to_string().contains("login required"),
            "{missing_token}"
        );
        let bad_scheme = model_from_parts("ftp://127.0.0.1", "token", "model").unwrap_err();
        assert!(bad_scheme.to_string().contains("invalid"), "{bad_scheme}");
        let missing_model = model_from_parts("http://127.0.0.1:1", "token", "").unwrap_err();
        assert!(
            missing_model.to_string().contains("model is missing"),
            "{missing_model}"
        );
    }

    #[test]
    fn model_parts_build_responses_model() {
        let model = model_from_parts("http://127.0.0.1:1/", "token", "gateway-model").unwrap();
        assert_eq!(model.id(), Some("gateway-model"));
        assert_eq!(model.name(), "openai");
    }

    #[test]
    fn unconfigured_model_is_a_placeholder_responses_model() {
        let model = unconfigured_model().unwrap();
        assert_eq!(model.id(), Some("unconfigured"));
        assert_eq!(model.name(), "openai");
    }

    #[test]
    fn model_from_config_rejects_local_providers() {
        let local = model_from_config("mock", "http://127.0.0.1:1", "token", "model").unwrap_err();
        assert!(
            local.to_string().contains("does not support provider"),
            "{local}"
        );
        let gateway = model_from_config("gateway", "http://127.0.0.1:1", "token", "model");
        assert!(gateway.is_ok());
    }
}
