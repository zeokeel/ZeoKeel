//! SSRF-guarded web fetch.
#![forbid(unsafe_code)]
use std::io::Read;
use std::net::{IpAddr, ToSocketAddrs};
use std::time::Duration;
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

const MAX_BYTES: u64 = 1024 * 1024;

pub struct WebFetchTool;
#[must_use] pub fn tool() -> WebFetchTool { WebFetchTool }

fn forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local() || v4.is_broadcast() || v4.is_unspecified() || v4.octets()[0] == 0 || (v4.octets()[0] == 100 && (64..=127).contains(&v4.octets()[1])) || (v4.octets()[0] == 192 && v4.octets()[1] == 0 && v4.octets()[2] == 0),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified() || v6.is_unique_local() || v6.is_unicast_link_local(),
    }
}

fn host_allowed(url: &reqwest::Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") { return false; }
    if url.username() != "" || url.password().is_some() { return false; }
    let Some(host) = url.host_str() else { return false; };
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".local") { return false; }
    if let Ok(ip) = host.parse::<IpAddr>() { return !forbidden_ip(ip); }
    let Ok(addresses) = (host, url.port_or_known_default().unwrap_or(443)).to_socket_addrs() else { return false; };
    let mut any = false;
    for address in addresses { any = true; if forbidden_ip(address.ip()) { return false; } }
    any
}

impl Tool for WebFetchTool {
    fn spec(&self) -> ToolSpec { ToolSpec { name: "web.fetch".to_owned(), description: "Fetch a public http(s) URL and return its text (max 1MB). Private/local hosts and redirects to them are rejected.".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"url":{"type":"string"}},"required":["url"],"additionalProperties":false}), mutating: false, requires_approval: false } }
    fn call(&self, _args: &str) -> Result<String, ToolError> { Err(ToolError::InvalidArgs("web.fetch requires JSON arguments".to_owned())) }
    fn call_json(&self, _context: &TurnContext, args: serde_json::Value) -> Result<String, ToolError> {
        let url = args.get("url").and_then(serde_json::Value::as_str).ok_or_else(|| ToolError::InvalidArgs("url".to_owned()))?;
        let parsed = reqwest::Url::parse(url).map_err(|error| ToolError::InvalidArgs(error.to_string()))?;
        if !host_allowed(&parsed) { return Err(ToolError::Denied("private/local/invalid URL is not allowed".to_owned())); }
        let policy = reqwest::redirect::Policy::custom(|attempt| {
            if host_allowed(attempt.url()) { attempt.follow() } else { attempt.stop() }
        });
        let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(20)).redirect(policy).build().map_err(|error| ToolError::Io(error.to_string()))?;
        let response = client.get(parsed).send().map_err(|error| ToolError::Io(error.to_string()))?;
        if !response.status().is_success() { return Err(ToolError::Io(format!("HTTP {}", response.status()))); }
        if response.content_length().is_some_and(|length| length > MAX_BYTES) { return Err(ToolError::Denied("response too large (max 1MB)".to_owned())); }
        let mut bytes = Vec::new();
        response.take(MAX_BYTES.saturating_add(1)).read_to_end(&mut bytes).map_err(|error| ToolError::Io(error.to_string()))?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_BYTES { return Err(ToolError::Denied("response too large (max 1MB)".to_owned())); }
        Ok(zk_kit_tools::truncate(String::from_utf8_lossy(&bytes).into_owned()))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
    #[test]
    fn rejects_private_hosts() {
        for url in ["http://127.0.0.1/", "http://172.20.0.1/", "http://10.0.0.1/", "http://192.168.1.1/", "http://[::1]/", "http://169.254.169.254/"] {
            let parsed = reqwest::Url::parse(url).unwrap();
            assert!(!super::host_allowed(&parsed), "{url}");
        }
    }
}
