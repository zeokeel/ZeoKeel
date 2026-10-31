//! Web search (DuckDuckGo-compatible instant-answer endpoint).
//!
//! Endpoint resolution is **explicit settings > `ZEOKEEL_SEARCH_URL` > built-in
//! default** (B11): the host publishes `AppSettings.web_search` through the
//! config contract's non-secret [`zk_contract_config::RuntimeSettings`] snapshot;
//! the env var stays the fallback for fresh installs / direct embedding. The
//! section carries no credential (keyless endpoint) and is https-only.
#![forbid(unsafe_code)]
use std::time::Duration;
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

/// Built-in DuckDuckGo instant-answer endpoint.
pub const DEFAULT_SEARCH_URL: &str = "https://api.duckduckgo.com/";
/// Built-in request timeout (matches `WebSearchSettings::default`).
pub const DEFAULT_TIMEOUT_MS: u64 = 15_000;

/// The effective endpoint and timeout for one search call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchEndpoint {
    /// Absolute https endpoint base URL.
    pub base_url: String,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
}

/// Resolve the endpoint: published settings > env > built-in default.
///
/// A blank `web_search.base_url` means "not overridden" (the UI's empty field),
/// so the env/default fallback still applies. The timeout comes from the same
/// section when published; there is no env fallback for it.
#[must_use]
pub fn resolve_endpoint() -> SearchEndpoint {
    let published = zk_contract_config::runtime_settings()
        .ok()
        .flatten()
        .and_then(|snapshot| snapshot.web_search);
    let settings_base = published
        .as_ref()
        .map(|settings| settings.base_url.trim().to_owned())
        .filter(|base| !base.is_empty());
    let base_url = settings_base
        .or_else(|| {
            std::env::var("ZEOKEEL_SEARCH_URL")
                .ok()
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| DEFAULT_SEARCH_URL.to_owned());
    let timeout_ms = published.map_or(DEFAULT_TIMEOUT_MS, |settings| {
        settings.effective_timeout_ms()
    });
    SearchEndpoint {
        base_url,
        timeout_ms,
    }
}

pub struct WebSearchTool;
#[must_use]
pub fn tool() -> WebSearchTool {
    WebSearchTool
}
impl Tool for WebSearchTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "web_search".to_owned(),
            description:
                "Search the web and return abstract/related links (external untrusted data)."
                    .to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"],"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "web_search requires JSON".to_owned(),
        ))
    }
    fn call_json(
        &self,
        _context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let query = args
            .get("query")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("query".to_owned()))?;
        let endpoint = resolve_endpoint();
        let url = reqwest::Url::parse(&endpoint.base_url)
            .map_err(|error| ToolError::InvalidArgs(error.to_string()))?;
        if url.scheme() != "https" {
            return Err(ToolError::Denied(
                "search endpoint must be https".to_owned(),
            ));
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_millis(endpoint.timeout_ms))
            .build()
            .map_err(|error| ToolError::Io(error.to_string()))?;
        let response = client
            .get(url)
            .query(&[("q", query), ("format", "json"), ("no_html", "1")])
            .send()
            .map_err(|error| ToolError::Io(error.to_string()))?;
        if !response.status().is_success() {
            return Err(ToolError::Io(format!("HTTP {}", response.status())));
        }
        let body: serde_json::Value = response
            .json()
            .map_err(|error| ToolError::Io(error.to_string()))?;
        let mut out = Vec::new();
        if let Some(abstract_text) = body.get("AbstractText").and_then(serde_json::Value::as_str) {
            if !abstract_text.is_empty() {
                out.push(serde_json::json!({"title":"Abstract","url":body.get("AbstractURL").and_then(serde_json::Value::as_str).unwrap_or(""),"snippet":abstract_text}));
            }
        }
        if let Some(topics) = body
            .get("RelatedTopics")
            .and_then(serde_json::Value::as_array)
        {
            for topic in topics {
                if out.len() >= 8 {
                    break;
                }
                if let Some(text) = topic.get("Text").and_then(serde_json::Value::as_str) {
                    out.push(serde_json::json!({"title":text.chars().take(80).collect::<String>(),"url":topic.get("FirstURL").and_then(serde_json::Value::as_str).unwrap_or(""),"snippet":text}));
                }
            }
        }
        Ok(serde_json::json!({"results":out,"note":"external untrusted data"}).to_string())
    }
}
