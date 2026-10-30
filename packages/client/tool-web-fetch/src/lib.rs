//! SSRF-guarded web fetch.
#![forbid(unsafe_code)]
use std::time::Duration;
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

pub struct WebFetchTool;
#[must_use] pub fn tool() -> WebFetchTool { WebFetchTool }
impl Tool for WebFetchTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "web.fetch".to_owned(), description: "Fetch a public http(s) URL and return its text (max 1MB). Private/local hosts are rejected.".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"url":{"type":"string"}},"required":["url"],"additionalProperties":false}), mutating: false, requires_approval: false } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::InvalidArgs("web.fetch requires JSON arguments".to_owned())) }
    fn call_json(&self, _context: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let url = args.get("url").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("url".to_owned()))?;
        let parsed = reqwest::Url::parse(url).map_err(|error| ToolError::InvalidArgs(error.to_string()))?;
        if !matches!(parsed.scheme(), "http" | "https") { return Err(ToolError::Denied("only http(s) URLs are allowed".to_owned())); }
        let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
        if host.is_empty() || host == "localhost" || host.ends_with(".local") || host.starts_with("127.") || host.starts_with("10.") || host.starts_with("192.168.") || host.starts_with("169.254.") || host.starts_with("172.16.") || host == "[::1]" {
            return Err(ToolError::Denied("private/local hosts are not allowed".to_owned()));
        }
        let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(20)).redirect(reqwest::redirect::Policy::limited(3)).build().map_err(|error| ToolError::Io(error.to_string()))?;
        let response = client.get(parsed).send().map_err(|error| ToolError::Io(error.to_string()))?;
        if !response.status().is_success() { return Err(ToolError::Io(format!("HTTP {}", response.status()))); }
        let text = response.text().map_err(|error| ToolError::Io(error.to_string()))?;
        Ok(zk_kit_tools::truncate(text))
    }
}
