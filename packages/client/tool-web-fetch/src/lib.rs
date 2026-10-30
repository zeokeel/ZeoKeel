//! SSRF-guarded web fetch.
#![forbid(unsafe_code)]
use std::io::Read;
use std::net::{IpAddr, ToSocketAddrs};
use std::time::Duration;
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

const MAX_BYTES: u64 = 1024 * 1024;

pub struct WebFetchTool;
#[must_use]
pub fn tool() -> WebFetchTool {
    WebFetchTool
}

fn forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || v4.octets()[0] == 0
                || (v4.octets()[0] == 100 && (64..=127).contains(&v4.octets()[1]))
                || (v4.octets()[0] == 192 && v4.octets()[1] == 0 && v4.octets()[2] == 0)
                // Benchmarking range (198.18.0.0/15) and reserved 240.0.0.0/4.
                || (v4.octets()[0] == 198 && (v4.octets()[1] & 0xfe) == 18)
                || v4.octets()[0] >= 240
        }
        IpAddr::V6(v6) => {
            // IPv4-mapped IPv6 (::ffff:a.b.c.d) resolves to an IPv4 endpoint; apply
            // the IPv4 policy first, otherwise loopback/private/metadata addresses
            // slip through as "public" IPv6.
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return forbidden_ip(IpAddr::V4(mapped));
            }
            // IPv4-compatible (deprecated) addresses map the same way.
            if let Some(compatible) = v6.to_ipv4() {
                return forbidden_ip(IpAddr::V4(compatible));
            }
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
        }
    }
}

fn resolve_public(url: &reqwest::Url) -> Option<std::net::SocketAddr> {
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    if url.username() != "" || url.password().is_some() {
        return None;
    }
    // `host_str()` returns IPv6 literals in brackets; strip them so they are
    // classified as addresses (not DNS names) before any network access.
    let host = url
        .host_str()?
        .trim_start_matches('[')
        .trim_end_matches(']');
    if host.eq_ignore_ascii_case("localhost") || host.to_ascii_lowercase().ends_with(".local") {
        return None;
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        return (!forbidden_ip(ip))
            .then(|| std::net::SocketAddr::new(ip, url.port_or_known_default().unwrap_or(443)));
    }
    let addresses: Vec<_> = (host, url.port_or_known_default().unwrap_or(443))
        .to_socket_addrs()
        .ok()?
        .collect();
    if addresses.is_empty() || addresses.iter().any(|address| forbidden_ip(address.ip())) {
        return None;
    }
    addresses.first().copied()
}

impl Tool for WebFetchTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec { name: "web.fetch".to_owned(), description: "Fetch a public http(s) URL and return its text (max 1MB). Private/local hosts and redirects to them are rejected.".to_owned(), parameters: serde_json::json!({"type":"object","properties":{"url":{"type":"string"}},"required":["url"],"additionalProperties":false}), mutating: false, requires_approval: false }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "web.fetch requires JSON arguments".to_owned(),
        ))
    }
    fn call_json(
        &self,
        _context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let input = args
            .get("url")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("url".to_owned()))?;
        let mut current = reqwest::Url::parse(input)
            .map_err(|error| ToolError::InvalidArgs(error.to_string()))?;
        for _ in 0..=3 {
            let Some(address) = resolve_public(&current) else {
                return Err(ToolError::Denied(
                    "private/local/invalid URL is not allowed".to_owned(),
                ));
            };
            let host = current
                .host_str()
                .ok_or_else(|| ToolError::InvalidArgs("URL has no host".to_owned()))?
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_owned();
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(20))
                .redirect(reqwest::redirect::Policy::none())
                .resolve(&host, address)
                .build()
                .map_err(|error| ToolError::Io(error.to_string()))?;
            let response = client
                .get(current.clone())
                .send()
                .map_err(|error| ToolError::Io(error.to_string()))?;
            if response.status().is_redirection() {
                let Some(location) = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                else {
                    return Err(ToolError::Io("redirect without location".to_owned()));
                };
                current = current
                    .join(location)
                    .map_err(|error| ToolError::InvalidArgs(error.to_string()))?;
                continue;
            }
            if !response.status().is_success() {
                return Err(ToolError::Io(format!("HTTP {}", response.status())));
            }
            if response
                .content_length()
                .is_some_and(|length| length > MAX_BYTES)
            {
                return Err(ToolError::Denied("response too large (max 1MB)".to_owned()));
            }
            let mut bytes = Vec::new();
            response
                .take(MAX_BYTES.saturating_add(1))
                .read_to_end(&mut bytes)
                .map_err(|error| ToolError::Io(error.to_string()))?;
            if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_BYTES {
                return Err(ToolError::Denied("response too large (max 1MB)".to_owned()));
            }
            return Ok(zk_kit_tools::truncate(
                String::from_utf8_lossy(&bytes).into_owned(),
            ));
        }
        Err(ToolError::Io("too many redirects".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
    #[test]
    fn rejects_private_hosts() {
        for url in [
            "http://127.0.0.1/",
            "http://172.20.0.1/",
            "http://10.0.0.1/",
            "http://192.168.1.1/",
            "http://[::1]/",
            "http://169.254.169.254/",
        ] {
            let parsed = reqwest::Url::parse(url).unwrap();
            assert!(super::resolve_public(&parsed).is_none(), "{url}");
        }
    }

    #[test]
    fn rejects_ipv4_mapped_private_hosts() {
        // SSRF bypass regression: ::ffff:a.b.c.d must be judged as IPv4.
        for url in [
            "http://[::ffff:127.0.0.1]/",
            "http://[::ffff:10.0.0.1]/",
            "http://[::ffff:172.20.0.1]/",
            "http://[::ffff:192.168.1.1]/",
            "http://[::ffff:169.254.169.254]/",
            "http://[::ffff:100.100.100.200]/",
            "http://[::ffff:198.18.0.1]/",
        ] {
            let parsed = reqwest::Url::parse(url).unwrap();
            assert!(super::resolve_public(&parsed).is_none(), "{url}");
        }
    }

    #[test]
    fn allows_public_ipv4_mapped_host() {
        let parsed = reqwest::Url::parse("http://[::ffff:8.8.8.8]/").unwrap();
        assert!(super::resolve_public(&parsed).is_some());
    }

    #[test]
    fn rejects_reserved_ipv4_ranges() {
        for url in [
            "http://198.18.0.1/",
            "http://198.19.255.254/",
            "http://240.0.0.1/",
        ] {
            let parsed = reqwest::Url::parse(url).unwrap();
            assert!(super::resolve_public(&parsed).is_none(), "{url}");
        }
    }
}
