//! Web search (DuckDuckGo instant answer).
#![forbid(unsafe_code)]
use std::time::Duration;
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};
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
        let base = std::env::var("ZEOKEEL_SEARCH_URL")
            .unwrap_or_else(|_| "https://api.duckduckgo.com/".to_owned());
        let url = reqwest::Url::parse(&base)
            .map_err(|error| ToolError::InvalidArgs(error.to_string()))?;
        if url.scheme() != "https" {
            return Err(ToolError::Denied(
                "search endpoint must be https".to_owned(),
            ));
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
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
