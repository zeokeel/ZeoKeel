//! Connector aggregate contract (`srv.connectors`).
//!
//! @capability contract.connector
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};

/// MCP transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorTransport {
    Stdio,
    Http,
    Sse,
}

/// Connector definition (no secret/token).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Connector {
    pub id: String,
    pub name: String,
    pub transport: ConnectorTransport,
    pub auth_mode: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub enabled: bool,
}

/// Connector grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConnectorGrant {
    pub grantee: String,
    pub can_enable: bool,
}

/// OAuth public-client metadata (no client_secret; ADR-060, docs/07 §6).
///
/// The desktop client runs the authorization-code + PKCE flow itself: it opens the
/// system browser at [`authorize_url`](Self::authorize_url), listens on an ephemeral
/// loopback redirect (RFC 8252), then exchanges the code at
/// [`token_url`](Self::token_url). Tokens stay in the client keychain; the server only
/// stores this non-secret metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConnectorOAuth {
    pub client_id: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub redirect_uri: String,
    /// OAuth authorize endpoint (non-secret; catalog `config.authorize_url`).
    #[serde(default)]
    pub authorize_url: String,
    /// OAuth token endpoint (non-secret; catalog `config.token_url`).
    #[serde(default)]
    pub token_url: String,
}

impl ConnectorOAuth {
    /// Validate the public-client metadata before a client starts the flow.
    ///
    /// Rules mirror the gateway SSRF policy (`docs/07` §6.2): endpoints must be
    /// `https://`, plain `http://` is accepted **only** for loopback hosts (local dev /
    /// MCP servers on the same machine), userinfo and fragments are rejected. The
    /// configured `redirect_uri` may be empty (the client then uses its own loopback
    /// URI); a non-loopback redirect URI is rejected because the desktop client is a
    /// native app and must use an RFC 8252 loopback redirect.
    ///
    /// # Errors
    ///
    /// A stable `OAUTH_VALIDATION: ...` message naming the offending field.
    pub fn validate(&self) -> Result<(), String> {
        if self.client_id.trim().is_empty() || self.client_id.len() > 256 {
            return Err("OAUTH_VALIDATION: client_id is required (<=256 bytes)".to_owned());
        }
        if self.client_id.chars().any(|c| c.is_control()) {
            return Err("OAUTH_VALIDATION: client_id contains control characters".to_owned());
        }
        if self.authorize_url.trim().is_empty() {
            return Err("OAUTH_VALIDATION: authorize_url is required".to_owned());
        }
        if self.token_url.trim().is_empty() {
            return Err("OAUTH_VALIDATION: token_url is required".to_owned());
        }
        validate_oauth_endpoint("authorize_url", &self.authorize_url)?;
        validate_oauth_endpoint("token_url", &self.token_url)?;
        if !self.redirect_uri.trim().is_empty() && !is_loopback_uri(&self.redirect_uri) {
            return Err(
                "OAUTH_VALIDATION: redirect_uri must be a loopback http URI (RFC 8252)".to_owned(),
            );
        }
        for scope in &self.scopes {
            if scope.trim().is_empty() || scope.chars().any(char::is_whitespace) {
                return Err(
                    "OAUTH_VALIDATION: scopes must be non-empty tokens without whitespace"
                        .to_owned(),
                );
            }
        }
        Ok(())
    }
}

/// Whether `raw` is an `http://` URI whose host is loopback.
///
/// Loopback is `localhost`, `127.0.0.0/8` and `[::1]`; the check is intentionally
/// textual (no DNS resolution) so it cannot be tricked by rebinding at validation time.
#[must_use]
pub fn is_loopback_uri(raw: &str) -> bool {
    let Some(rest) = raw.trim().strip_prefix("http://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.contains('@') {
        // Userinfo never belongs in a loopback redirect URI (RFC 8252).
        return false;
    }
    let host_port = authority;
    let host = if let Some(stripped) = host_port.strip_prefix('[') {
        stripped.split(']').next().unwrap_or_default()
    } else {
        host_port.split(':').next().unwrap_or_default()
    };
    host.eq_ignore_ascii_case("localhost")
        || host == "::1"
        || host == "0:0:0:0:0:0:0:1"
        || host.starts_with("127.")
}

/// Validate one OAuth endpoint URL (https anywhere; http only for loopback).
fn validate_oauth_endpoint(field: &str, raw: &str) -> Result<(), String> {
    let raw = raw.trim();
    if raw.contains(char::is_whitespace) || raw.contains('#') {
        return Err(format!(
            "OAUTH_VALIDATION: {field} must be a URL without whitespace/fragment"
        ));
    }
    let Some((scheme, rest)) = raw.split_once("://") else {
        return Err(format!("OAUTH_VALIDATION: {field} must be an absolute URL"));
    };
    let rest_without_scheme = rest;
    let authority = rest_without_scheme
        .split(['/', '?'])
        .next()
        .unwrap_or_default();
    if authority.is_empty() {
        return Err(format!("OAUTH_VALIDATION: {field} has no host"));
    }
    if authority.contains('@') {
        return Err(format!("OAUTH_VALIDATION: {field} must not embed userinfo"));
    }
    match scheme {
        "https" => Ok(()),
        "http" if is_loopback_uri(raw) => Ok(()),
        "http" => Err(format!(
            "OAUTH_VALIDATION: {field} must use https:// (http:// is loopback-only)"
        )),
        other => Err(format!(
            "OAUTH_VALIDATION: {field} must be http(s), got scheme `{other}`"
        )),
    }
}

/// MCP protocol version implemented by the built-in client transport.
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Client-side MCP server configuration (non-secret; ADR-060).
///
/// Delivered by the connector catalog / local settings; the client spawns `command`
/// (stdio) or POSTs to `url` (http/sse). Static secrets are rejected by validation
/// (`env` keys must not look like credentials); OAuth tokens stay in the OS keychain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct McpServerConfig {
    /// Connector id (stable, used by tools as `server` argument).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Transport.
    pub transport: ConnectorTransport,
    /// Executable for `transport = "stdio"` (no shell; args are passed verbatim).
    #[serde(default)]
    pub command: String,
    /// Verbatim argv for stdio (no shell concatenation).
    #[serde(default)]
    pub args: Vec<String>,
    /// Allow-listed environment for the child process (no static secrets).
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    /// HTTP(S) endpoint for `transport = "http"` / `"sse"`. Plain `http://` is accepted
    /// only for loopback/dev hosts (threat model, docs/07 §6.2); non-loopback requires `https://`.
    #[serde(default)]
    pub url: String,
    /// Per-request timeout in milliseconds (0 or absent => default 10s).
    #[serde(default)]
    pub timeout_ms: u64,
    /// Whether this server requires an OAuth bearer token (client-side keychain).
    ///
    /// Set from the catalog's public-client metadata; the transport injects
    /// `Authorization: Bearer <token>` only when a token was installed by the
    /// client OAuth flow and fails honestly with `MCP_OAUTH_REQUIRED` otherwise.
    #[serde(default)]
    pub oauth_required: bool,
}

/// Non-secret connector execution config delivered to clients (ADR-060).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConnectorConfig {
    /// Executable for stdio (no shell; args verbatim).
    #[serde(default)]
    pub command: String,
    /// Verbatim argv for stdio.
    #[serde(default)]
    pub args: Vec<String>,
    /// Allow-listed child environment (no static secrets).
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    /// HTTP(S) endpoint for http/sse transports.
    #[serde(default)]
    pub url: String,
    /// Per-request timeout (ms; 0/absent => client default).
    #[serde(default)]
    pub timeout_ms: u64,
}

/// One client connector catalog item (`GET /api/client/v1/connectors`, extended with `config`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConnectorCatalogItem {
    pub id: String,
    pub name: String,
    pub transport: ConnectorTransport,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub config: ConnectorConfig,
    /// OAuth public-client metadata (no secret); absent for non-OAuth connectors.
    #[serde(default)]
    pub oauth: Option<ConnectorOAuth>,
}

/// Longest accepted MCP URL.
const MAX_URL_CHARS: usize = 2048;
/// Env-key markers rejected as static secrets (ADR-060).
const SECRET_ENV_MARKERS: [&str; 6] = [
    "secret",
    "token",
    "password",
    "passwd",
    "api_key",
    "credential",
];

fn url_host(url: &str) -> Option<&str> {
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))?;
    let authority = rest
        .split(['/', '?', '#'])
        .next()
        .filter(|value| !value.is_empty())?;
    let host_port = match authority.rsplit_once('@') {
        Some((_, host)) => host,
        None => authority,
    };
    let host = match host_port.strip_prefix('[') {
        Some(rest) => rest.split(']').next()?,
        None => host_port.split(':').next()?,
    };
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1") || host.starts_with("127.")
}

impl McpServerConfig {
    /// Validate a client-side MCP server definition (non-secret config, ADR-060).
    ///
    /// This is the single implementation shared by the catalog path
    /// (`zk-connector-mcp::servers_from_catalog`) and the local-settings path
    /// (`McpSettings::validate` calls this + local caps). Rules mirror the previous
    /// client-side code verbatim: id/name required, env keys must not look like
    /// static secrets, stdio requires `command`, http/sse require a bounded
    /// whitespace-free URL, and plain `http://` is loopback-only (threat model,
    /// `docs/07` §6.2).
    ///
    /// # Errors
    ///
    /// Stable `MCP_VALIDATION:` / `MCP_INSECURE_TRANSPORT:` messages.
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("MCP_VALIDATION: id is empty".to_owned());
        }
        if self.name.trim().is_empty() {
            return Err("MCP_VALIDATION: name is empty".to_owned());
        }
        for key in self.env.keys() {
            let lowered = key.to_ascii_lowercase();
            if SECRET_ENV_MARKERS
                .iter()
                .any(|marker| lowered.contains(marker))
            {
                return Err(format!(
                    "MCP_VALIDATION: env key {key} looks like a static secret; ADR-060 forbids static secrets in connector config"
                ));
            }
        }
        match self.transport {
            ConnectorTransport::Stdio => {
                if self.command.trim().is_empty() {
                    return Err("MCP_VALIDATION: stdio transport requires command".to_owned());
                }
            }
            ConnectorTransport::Http | ConnectorTransport::Sse => {
                let url = self.url.trim();
                if url.is_empty() {
                    return Err("MCP_VALIDATION: http transport requires url".to_owned());
                }
                if url.chars().count() > MAX_URL_CHARS {
                    return Err("MCP_VALIDATION: url is too long".to_owned());
                }
                if url.chars().any(|ch| ch.is_whitespace() || ch.is_control()) {
                    return Err(
                        "MCP_VALIDATION: url contains whitespace/control characters".to_owned()
                    );
                }
                let Some(host) = url_host(url) else {
                    return Err(
                        "MCP_VALIDATION: url must start with http:// or https://".to_owned()
                    );
                };
                if url.starts_with("http://") && !is_loopback_host(host) {
                    return Err(format!(
                        "MCP_INSECURE_TRANSPORT: plain http:// is only allowed for loopback/dev hosts (got {host}); use https:// or stdio (threat model, docs/07 §6.2)"
                    ));
                }
            }
        }
        Ok(())
    }
}

impl ConnectorCatalogItem {
    /// Map a catalog item to the client MCP server config.
    #[must_use]
    pub fn into_mcp_server(self) -> McpServerConfig {
        let oauth_required = self.oauth.is_some();
        McpServerConfig {
            id: self.id,
            name: self.name,
            transport: self.transport,
            command: self.config.command,
            args: self.config.args,
            env: self.config.env,
            url: self.config.url,
            timeout_ms: self.config.timeout_ms,
            oauth_required,
        }
    }
}
