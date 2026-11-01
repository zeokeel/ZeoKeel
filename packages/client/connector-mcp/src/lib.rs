//! MCP (Model Context Protocol) client transport for connectors (B16/R9.6).
//!
//! Implements JSON-RPC `initialize` / `tools/list` / `tools/call` over stdio
//! (spawned child, line-delimited JSON-RPC) and HTTP(S) POST (rustls TLS; plain
//! `http://` only for loopback/dev hosts), and exposes `mcp_call` /
//! `mcp_list_tools` through the existing agent tool registry.
//!
//! Composition stays build-time (C2): this package is a regular composed tool package
//! (role `tool.mcp`); the generator appends [`extra_tools`] automatically. No runtime
//! plugin loading. Configuration is non-secret (`McpServerConfig` / connector catalog,
//! ADR-060) and can be delivered by the connector catalog ([`update_from_catalog`]) or
//! local `ZEOKEEL_MCP_SERVERS`; sessions are initialized once per connector and reused.
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::Duration;

use serde_json::{json, Value};
use zk_contract_connector::{
    ConnectorCatalogItem, ConnectorTransport, McpServerConfig, MCP_PROTOCOL_VERSION,
};
use zk_contract_tool::{ApprovalMetadata, ApprovalRisk, Tool, ToolError, ToolSpec, TurnContext};

/// Environment variable holding a JSON array of [`McpServerConfig`] (local fallback).
pub const SERVERS_ENV: &str = "ZEOKEEL_MCP_SERVERS";

/// Default per-request timeout when `timeout_ms` is absent/zero.
pub const DEFAULT_TIMEOUT_MS: u64 = 10_000;

const MAX_MESSAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_ERROR_CHARS: usize = 500;
fn timeout_of(server: &McpServerConfig) -> Duration {
    let ms = if server.timeout_ms == 0 {
        DEFAULT_TIMEOUT_MS
    } else {
        server.timeout_ms
    };
    Duration::from_millis(ms)
}

fn limit_chars(text: &str) -> String {
    if text.chars().count() <= MAX_ERROR_CHARS {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(MAX_ERROR_CHARS).collect();
    out.push('…');
    out
}

fn state_error() -> String {
    "MCP_STATE: connector/session lock poisoned".to_owned()
}

/// Extract the host part of an `http(s)://` URL.
/// Validate a client-side MCP server definition (non-secret config, ADR-060).
///
/// # Errors
///
/// Returns `MCP_VALIDATION:` / `MCP_INSECURE_TRANSPORT:` messages for invalid or
/// unsafe configurations.
pub fn validate_server(server: &McpServerConfig) -> Result<(), String> {
    // Single implementation lives in the connector contract (C17); both the
    // catalog path and the local-settings path validate through this delegate.
    server.validate()
}

/// Parse/validate a JSON array of [`McpServerConfig`] (used by [`servers_from_env`]).
///
/// # Errors
///
/// Returns `MCP_CONFIG:` for unparsable JSON and propagates [`validate_server`] errors.
pub fn parse_servers(raw: &str) -> Result<Vec<McpServerConfig>, String> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let servers: Vec<McpServerConfig> = serde_json::from_str(raw)
        .map_err(|error| format!("MCP_CONFIG: {SERVERS_ENV} is not a JSON array: {error}"))?;
    for server in &servers {
        validate_server(server)?;
    }
    Ok(servers)
}

/// Load configured MCP servers from the client-local [`SERVERS_ENV`] variable.
///
/// # Errors
///
/// Returns configuration/validation errors as strings.
pub fn servers_from_env() -> Result<Vec<McpServerConfig>, String> {
    let raw = std::env::var(SERVERS_ENV).unwrap_or_default();
    parse_servers(&raw)
}

/// Map an enterprise connector catalog response (`{"items": [...]}`) to MCP servers.
///
/// Only `enabled` items are exposed; secret-looking `env` keys are rejected by
/// [`validate_server`] (ADR-060). The catalog must include the non-secret execution
/// `config` (stdio command/args/env or http(s) url) for a connector to be executable.
///
/// # Errors
///
/// Returns `MCP_CONFIG:` / `MCP_VALIDATION:` errors for invalid catalog payloads.
pub fn servers_from_catalog(catalog_json: &str) -> Result<Vec<McpServerConfig>, String> {
    let payload: Value = serde_json::from_str(catalog_json)
        .map_err(|error| format!("MCP_CONFIG: connector catalog is not JSON: {error}"))?;
    let items = match payload.get("items") {
        Some(items) => serde_json::from_value::<Vec<ConnectorCatalogItem>>(items.clone())
            .map_err(|error| format!("MCP_CONFIG: connector catalog items: {error}"))?,
        None => Vec::new(),
    };
    let mut servers = Vec::new();
    for item in items {
        if !item.enabled {
            continue;
        }
        let server = item.into_mcp_server();
        validate_server(&server)?;
        servers.push(server);
    }
    Ok(servers)
}

static CATALOG_SERVERS: OnceLock<Mutex<Option<Vec<McpServerConfig>>>> = OnceLock::new();

fn catalog_slot() -> &'static Mutex<Option<Vec<McpServerConfig>>> {
    CATALOG_SERVERS.get_or_init(|| Mutex::new(None))
}

/// Install/refresh the client connector config from the enterprise catalog payload.
///
/// This is the client side of the server connector-definition passthrough; it also
/// acts as the force-update hook (sessions are dropped and lazily re-initialized).
///
/// # Errors
///
/// Returns catalog validation errors or session-lock errors.
pub fn update_from_catalog(catalog_json: &str) -> Result<usize, String> {
    let servers = servers_from_catalog(catalog_json)?;
    let mut slot = catalog_slot().lock().map_err(|_| state_error())?;
    *slot = Some(servers.clone());
    drop(slot);
    let ids: Vec<String> = servers.iter().map(|server| server.id.clone()).collect();
    retain_oauth_tokens(&ids)?;
    reload_connectors()?;
    Ok(servers.len())
}

/// Drop catalog-provided config (fall back to local [`SERVERS_ENV`]).
///
/// # Errors
///
/// Returns session-lock errors.
pub fn clear_catalog_config() -> Result<(), String> {
    let mut slot = catalog_slot().lock().map_err(|_| state_error())?;
    *slot = None;
    drop(slot);
    clear_oauth_tokens()?;
    reload_connectors()
}

static LOCAL_SERVERS: OnceLock<Mutex<Option<Vec<McpServerConfig>>>> = OnceLock::new();
static ENV_FALLBACK: OnceLock<Mutex<bool>> = OnceLock::new();

fn local_slot() -> &'static Mutex<Option<Vec<McpServerConfig>>> {
    LOCAL_SERVERS.get_or_init(|| Mutex::new(None))
}

fn env_fallback_slot() -> &'static Mutex<bool> {
    ENV_FALLBACK.get_or_init(|| Mutex::new(true))
}

/// Install the personal local-settings MCP servers (`Some([])` = explicitly none).
///
/// Local sources (including an explicit empty list) replace the env fallback; the
/// catalog always wins over both (enterprise catalog-only, `docs/07` §6.1).
///
/// # Errors
///
/// Returns `MCP_VALIDATION:` errors or state-lock errors.
pub fn update_local_servers(servers: Vec<McpServerConfig>) -> Result<(), String> {
    for server in &servers {
        validate_server(server)?;
    }
    let mut slot = local_slot().lock().map_err(|_| state_error())?;
    *slot = Some(servers);
    drop(slot);
    let mut fallback = env_fallback_slot().lock().map_err(|_| state_error())?;
    *fallback = false;
    drop(fallback);
    reload_connectors()
}

/// Drop local-settings servers and re-enable the `ZEOKEEL_MCP_SERVERS` fallback
/// (fresh install / no settings file).
///
/// # Errors
///
/// Returns state-lock errors.
pub fn clear_local_servers() -> Result<(), String> {
    let mut slot = local_slot().lock().map_err(|_| state_error())?;
    *slot = None;
    drop(slot);
    let mut fallback = env_fallback_slot().lock().map_err(|_| state_error())?;
    *fallback = true;
    drop(fallback);
    reload_connectors()
}

/// Toggle the env fallback explicitly (e2e/tests; boot keeps the default).
///
/// # Errors
///
/// Returns state-lock errors.
pub fn set_env_fallback_enabled(enabled: bool) -> Result<(), String> {
    let mut fallback = env_fallback_slot().lock().map_err(|_| state_error())?;
    *fallback = enabled;
    drop(fallback);
    reload_connectors()
}

/// Source priority (`docs/07` §6.1 / spec R3): catalog (even empty) > local
/// settings (even empty) > env (only while the env fallback is enabled).
fn configured_servers() -> Result<Vec<McpServerConfig>, String> {
    let catalog = catalog_slot().lock().map_err(|_| state_error())?.clone();
    if let Some(servers) = catalog {
        return Ok(servers);
    }
    let local = local_slot().lock().map_err(|_| state_error())?.clone();
    if let Some(servers) = local {
        return Ok(servers);
    }
    let fallback = *env_fallback_slot().lock().map_err(|_| state_error())?;
    if fallback {
        servers_from_env()
    } else {
        Ok(Vec::new())
    }
}

/// One-shot, no-cache MCP probe used by `mcp.test` (never touches [`SESSIONS`]).
///
/// Spawns a fresh transport, runs `initialize` + `tools/list`, returns the tool
/// names (capped at 200, each <=128 chars), then drops the transport (stdio child
/// killed). Never installs a global session or OAuth token.
///
/// # Errors
///
/// Propagates validation/spawn/connect/protocol errors (`MCP_*` codes).
pub fn test_server(server: &McpServerConfig) -> Result<Vec<String>, String> {
    validate_server(server)?;
    let mut transport = transport(server)?;
    let _ = initialize(&mut transport)?;
    let result = transport.request("tools/list", json!({}))?;
    let tools = result
        .get("tools")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("name").and_then(Value::as_str))
                .take(200)
                .map(|name| name.chars().take(128).collect::<String>())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    drop(transport);
    Ok(tools)
}

/// Number of cached global MCP sessions (test/verification accessor only).
#[must_use]
pub fn session_count() -> usize {
    sessions().lock().map(|map| map.len()).unwrap_or(0)
}

// -------------------------------------------------------------- sessions ----

struct CachedSession {
    fingerprint: String,
    transport: ClientTransport,
}

static SESSIONS: OnceLock<Mutex<HashMap<String, Arc<Mutex<CachedSession>>>>> = OnceLock::new();

fn sessions() -> &'static Mutex<HashMap<String, Arc<Mutex<CachedSession>>>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_session(session: &Mutex<CachedSession>) -> Result<MutexGuard<'_, CachedSession>, String> {
    session.lock().map_err(|_| state_error())
}

/// Configured MCP server ids (catalog first, local env fallback).
///
/// # Errors
///
/// Returns configuration/validation errors as strings.
pub fn configured_server_ids() -> Result<Vec<String>, String> {
    Ok(configured_servers()?
        .into_iter()
        .map(|server| server.id)
        .collect())
}

/// Drop all MCP sessions (force-update hook; the next call re-initializes lazily).
///
/// # Errors
///
/// Returns session-lock errors.
pub fn reload_connectors() -> Result<(), String> {
    let mut map = sessions().lock().map_err(|_| state_error())?;
    map.clear();
    Ok(())
}

// --------------------------------------------------------- oauth tokens ----

/// Client-side OAuth bearer tokens per MCP server (in memory only; the durable
/// copy stays in the OS keychain via the desktop `oauth.save` IPC, never here).
///
/// No `Debug`/serialization is derived for this map so a token can never be
/// formatted into a log or an error message.
static OAUTH_TOKENS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn oauth_slot() -> &'static Mutex<HashMap<String, String>> {
    OAUTH_TOKENS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn valid_server_id(server_id: &str) -> bool {
    !server_id.is_empty()
        && server_id.len() <= 128
        && server_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Install (or clear with `None`) the OAuth bearer token for one MCP server.
///
/// The desktop calls this after its PKCE token exchange/refresh, and clears it on
/// logout/disconnect. Only `https`/loopback-`http` MCP servers can require OAuth;
/// the token is attached as `Authorization: Bearer ...` to that server's requests.
///
/// # Errors
///
/// `MCP_VALIDATION:` for an invalid server id or malformed token value.
pub fn set_oauth_token(server_id: &str, token: Option<String>) -> Result<(), String> {
    if !valid_server_id(server_id) {
        return Err("MCP_VALIDATION: invalid server id".to_owned());
    }
    let mut map = oauth_slot().lock().map_err(|_| state_error())?;
    match token {
        Some(token) => {
            let token = token.trim();
            if token.is_empty() {
                return Err("MCP_VALIDATION: oauth token is empty".to_owned());
            }
            if token.len() > 8_192 || token.chars().any(|c| c.is_control() || c.is_whitespace()) {
                return Err("MCP_VALIDATION: oauth token contains invalid characters".to_owned());
            }
            map.insert(server_id.to_owned(), token.to_owned());
        }
        None => {
            map.remove(server_id);
        }
    }
    Ok(())
}

/// Drop every in-memory OAuth token (logout / disconnect).
///
/// # Errors
///
/// Returns `MCP_STATE:` when the token registry lock is poisoned.
pub fn clear_oauth_tokens() -> Result<(), String> {
    let mut map = oauth_slot().lock().map_err(|_| state_error())?;
    map.clear();
    Ok(())
}

/// Whether an OAuth token is currently installed for `server_id` (no token value).
#[must_use]
pub fn has_oauth_token(server_id: &str) -> bool {
    oauth_slot()
        .lock()
        .map(|map| map.contains_key(server_id))
        .unwrap_or(false)
}

fn oauth_token(server_id: &str) -> Option<String> {
    oauth_slot()
        .lock()
        .ok()
        .and_then(|map| map.get(server_id).cloned())
}

/// Keep only tokens for servers that are still configured (catalog force-update).
fn retain_oauth_tokens(server_ids: &[String]) -> Result<(), String> {
    let mut map = oauth_slot().lock().map_err(|_| state_error())?;
    map.retain(|id, _token| server_ids.iter().any(|server| server == id));
    Ok(())
}

fn session_for(server: &McpServerConfig) -> Result<Arc<Mutex<CachedSession>>, String> {
    validate_server(server)?;
    let fingerprint = serde_json::to_string(server)
        .map_err(|error| format!("MCP_CONFIG: fingerprint: {error}"))?;
    let mut map = sessions().lock().map_err(|_| state_error())?;
    if let Some(existing) = map.get(&server.id).cloned() {
        let stale = {
            let guard = lock_session(existing.as_ref())?;
            guard.fingerprint != fingerprint
        };
        if !stale {
            return Ok(existing);
        }
        map.remove(&server.id);
    }
    let mut transport = transport(server)?;
    let _ = initialize(&mut transport)?;
    let session = Arc::new(Mutex::new(CachedSession {
        fingerprint,
        transport,
    }));
    map.insert(server.id.clone(), Arc::clone(&session));
    Ok(session)
}

fn with_session<T>(
    server: &McpServerConfig,
    call: impl FnOnce(&mut ClientTransport) -> Result<T, String>,
) -> Result<T, String> {
    let session = session_for(server)?;
    let mut guard = lock_session(session.as_ref())?;
    call(&mut guard.transport)
}

fn transport(server: &McpServerConfig) -> Result<ClientTransport, String> {
    match server.transport {
        ConnectorTransport::Stdio => Ok(ClientTransport::Stdio(StdioTransport::spawn(server)?)),
        ConnectorTransport::Http | ConnectorTransport::Sse => {
            Ok(ClientTransport::Http(HttpTransport::new(server)?))
        }
    }
}

fn initialize(transport: &mut ClientTransport) -> Result<Value, String> {
    let result = transport.request(
        "initialize",
        json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {"name": "zeokeel", "version": env!("CARGO_PKG_VERSION")}
        }),
    )?;
    transport.notify("notifications/initialized", json!({}))?;
    Ok(result)
}

/// Run `tools/list` against one MCP server (session initialized once and reused).
///
/// # Errors
///
/// Propagates validation, spawn/connect, timeout, protocol and JSON-RPC errors.
pub fn list_server_tools(server: &McpServerConfig) -> Result<Value, String> {
    with_session(server, |client| client.request("tools/list", json!({})))
}

/// Run `tools/call` against one MCP server (session initialized once and reused).
///
/// # Errors
///
/// Propagates validation, transport and JSON-RPC errors; a tool result with
/// `isError: true` becomes `MCP_TOOL_ERROR:`.
pub fn call_server_tool(
    server: &McpServerConfig,
    tool: &str,
    arguments: &Value,
) -> Result<Value, String> {
    if tool.trim().is_empty() {
        return Err("MCP_VALIDATION: tool name is empty".to_owned());
    }
    let result = with_session(server, |client| {
        client.request("tools/call", json!({"name": tool, "arguments": arguments}))
    })?;
    if result.get("isError").and_then(Value::as_bool) == Some(true) {
        return Err(format!(
            "MCP_TOOL_ERROR: {}",
            limit_chars(&result.to_string())
        ));
    }
    Ok(result)
}

fn unavailable() -> String {
    format!(
        "MCP_CAPABILITY_UNAVAILABLE: no MCP servers configured (connector catalog / {SERVERS_ENV}); catalog config passthrough may not be wired"
    )
}

/// All configured servers with their tool lists; per-server failures are reported
/// inline instead of aborting the whole listing.
///
/// # Errors
///
/// Returns `MCP_CAPABILITY_UNAVAILABLE:` when no server is configured and
/// `MCP_CONFIG:` for an invalid configuration.
pub fn list_all_servers() -> Result<Value, String> {
    let servers = configured_servers()?;
    if servers.is_empty() {
        return Err(unavailable());
    }
    let mut items = Vec::new();
    for server in &servers {
        let entry = match list_server_tools(server) {
            Ok(result) => json!({
                "id": server.id,
                "name": server.name,
                "transport": server.transport,
                "tools": result.get("tools").cloned().unwrap_or_else(|| json!([])),
            }),
            Err(error) => json!({
                "id": server.id,
                "name": server.name,
                "transport": server.transport,
                "error": limit_chars(&error),
            }),
        };
        items.push(entry);
    }
    Ok(json!({"servers": items}))
}

fn find_server(id: &str) -> Result<McpServerConfig, String> {
    let servers = configured_servers()?;
    if servers.is_empty() {
        return Err(unavailable());
    }
    servers
        .into_iter()
        .find(|server| server.id == id)
        .ok_or_else(|| format!("MCP_CAPABILITY_UNAVAILABLE: unknown MCP server {id}"))
}

enum ClientTransport {
    Stdio(StdioTransport),
    Http(HttpTransport),
}

impl ClientTransport {
    fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        match self {
            Self::Stdio(client) => client.request(method, params),
            Self::Http(client) => client.request(method, params),
        }
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        match self {
            Self::Stdio(client) => client.notify(method, params),
            Self::Http(client) => client.notify(method, params),
        }
    }
}

// ---------------------------------------------------------------- stdio ----

enum StdioEvent {
    Line(String),
    Error(String),
    Eof,
}

struct StdioTransport {
    child: Child,
    stdin: ChildStdin,
    events: Receiver<StdioEvent>,
    next_id: u64,
    timeout: Duration,
}

impl StdioTransport {
    fn spawn(server: &McpServerConfig) -> Result<Self, String> {
        let mut command = Command::new(&server.command);
        command
            .args(&server.args)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for (key, value) in &server.env {
            command.env(key, value);
        }
        // Allow-list: only explicit config env, plus PATH so the executable can be resolved.
        if !server.env.contains_key("PATH") {
            if let Ok(path) = std::env::var("PATH") {
                command.env("PATH", path);
            }
        }
        let mut child = command
            .spawn()
            .map_err(|error| format!("MCP_SPAWN: {}: {error}", server.command))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "MCP_SPAWN: stdin pipe unavailable".to_owned())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "MCP_SPAWN: stdout pipe unavailable".to_owned())?;
        let (sender, events) = mpsc::channel();
        let _reader = std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        let _ = sender.send(StdioEvent::Eof);
                        break;
                    }
                    Ok(read) if read > MAX_MESSAGE_BYTES => {
                        let _ = sender.send(StdioEvent::Error(
                            "MCP_PROTOCOL: message too large".to_owned(),
                        ));
                        break;
                    }
                    Ok(_) => {
                        let trimmed = line.trim_end().to_owned();
                        if !trimmed.is_empty() && sender.send(StdioEvent::Line(trimmed)).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = sender.send(StdioEvent::Error(format!("MCP_IO: {error}")));
                        break;
                    }
                }
            }
        });
        Ok(Self {
            child,
            stdin,
            events,
            next_id: 0,
            timeout: timeout_of(server),
        })
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| "MCP_PROTOCOL: request id overflow".to_owned())?;
        self.next_id = id;
        self.write_message(
            &json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}),
        )?;
        loop {
            match self.events.recv_timeout(self.timeout) {
                Ok(StdioEvent::Line(line)) => {
                    let value: Value = serde_json::from_str(&line)
                        .map_err(|error| format!("MCP_PROTOCOL: invalid JSON response: {error}"))?;
                    if value.get("id").and_then(Value::as_u64) == Some(id) {
                        return jsonrpc_result(value);
                    }
                    // Notifications and other ids are ignored.
                }
                Ok(StdioEvent::Error(error)) => return Err(error),
                Ok(StdioEvent::Eof) => {
                    return Err("MCP_EOF: server closed stdout before responding".to_owned());
                }
                Err(_) => return Err("MCP_TIMEOUT: no response within timeout".to_owned()),
            }
        }
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        self.write_message(&json!({"jsonrpc": "2.0", "method": method, "params": params}))
    }

    fn write_message(&mut self, payload: &Value) -> Result<(), String> {
        let text = serde_json::to_string(payload)
            .map_err(|error| format!("MCP_PROTOCOL: encode request: {error}"))?;
        self.stdin
            .write_all(text.as_bytes())
            .map_err(|error| format!("MCP_IO: write stdin: {error}"))?;
        self.stdin
            .write_all(b"\n")
            .map_err(|error| format!("MCP_IO: write stdin: {error}"))?;
        self.stdin
            .flush()
            .map_err(|error| format!("MCP_IO: flush stdin: {error}"))
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ----------------------------------------------------------------- http ----

struct HttpTransport {
    client: reqwest::blocking::Client,
    url: String,
    server_id: String,
    oauth_required: bool,
    next_id: u64,
    session_id: Option<String>,
}

impl HttpTransport {
    fn new(server: &McpServerConfig) -> Result<Self, String> {
        let timeout = timeout_of(server);
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .connect_timeout(timeout)
            .build()
            .map_err(|error| format!("MCP_HTTP: build client: {error}"))?;
        Ok(Self {
            client,
            url: server.url.trim().to_owned(),
            server_id: server.id.clone(),
            oauth_required: server.oauth_required,
            next_id: 0,
            session_id: None,
        })
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| "MCP_PROTOCOL: request id overflow".to_owned())?;
        self.next_id = id;
        let payload = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let body = serde_json::to_string(&payload)
            .map_err(|error| format!("MCP_PROTOCOL: encode request: {error}"))?;
        // OAuth-gated server: inject the client-held bearer token, or fail honestly.
        // Non-OAuth servers never receive an Authorization header, even if a stale
        // token exists for the same id.
        let oauth_token = self
            .oauth_required
            .then(|| oauth_token(&self.server_id))
            .flatten();
        if self.oauth_required && oauth_token.is_none() {
            return Err(format!(
                "MCP_OAUTH_REQUIRED: connector `{}` requires OAuth; authorize it first",
                self.server_id
            ));
        }
        let mut request = self
            .client
            .post(&self.url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(
                reqwest::header::ACCEPT,
                "application/json, text/event-stream",
            )
            .body(body);
        if let Some(token) = oauth_token {
            request = request.bearer_auth(token);
        }
        if let Some(session) = &self.session_id {
            request = request.header("Mcp-Session-Id", session);
        }
        let response = request
            .send()
            .map_err(|error| format!("MCP_HTTP: request failed: {error}"))?;
        let status = response.status().as_u16();
        let session = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let body = response
            .text()
            .map_err(|error| format!("MCP_HTTP: read body: {error}"))?;
        if status != 200 {
            return Err(format!("MCP_HTTP_STATUS: {status}"));
        }
        if let Some(value) = session {
            self.session_id = Some(value);
        }
        if content_type.contains("text/event-stream") {
            return sse_result(&body, id);
        }
        let value: Value = serde_json::from_str(body.trim())
            .map_err(|error| format!("MCP_PROTOCOL: invalid JSON response body: {error}"))?;
        if value.get("id").and_then(Value::as_u64) == Some(id) {
            jsonrpc_result(value)
        } else {
            Err("MCP_PROTOCOL: response id mismatch".to_owned())
        }
    }

    fn notify(&mut self, _method: &str, _params: Value) -> Result<(), String> {
        // Streamable-HTTP notifications are fire-and-forget; tools/list/tools/call
        // do not require the initialized notification to be acknowledged.
        Ok(())
    }
}

fn sse_result(body: &str, id: u64) -> Result<Value, String> {
    for line in body.split('\n') {
        let line = line.trim_end();
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        let value: Value = serde_json::from_str(data)
            .map_err(|error| format!("MCP_PROTOCOL: invalid JSON SSE data: {error}"))?;
        if value.get("id").and_then(Value::as_u64) == Some(id) {
            return jsonrpc_result(value);
        }
    }
    Err("MCP_PROTOCOL: SSE response has no matching JSON-RPC result".to_owned())
}

fn jsonrpc_result(value: Value) -> Result<Value, String> {
    if let Some(error) = value.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown JSON-RPC error");
        return Err(format!("MCP_RPC_ERROR: {}", limit_chars(message)));
    }
    value
        .get("result")
        .cloned()
        .ok_or_else(|| "MCP_PROTOCOL: response without result".to_owned())
}

// ---------------------------------------------------------------- tools ----

fn tool_error(error: String) -> ToolError {
    if error.starts_with("MCP_VALIDATION:") {
        ToolError::InvalidArgs(limit_chars(&error))
    } else if error.starts_with("MCP_CAPABILITY_UNAVAILABLE:")
        || error.starts_with("MCP_INSECURE_TRANSPORT:")
        || error.starts_with("MCP_CONFIG:")
        || error.starts_with("MCP_TOOL_ERROR:")
        || error.starts_with("MCP_RPC_ERROR:")
        || error.starts_with("MCP_STATE:")
    {
        ToolError::Denied(limit_chars(&error))
    } else {
        ToolError::Io(limit_chars(&error))
    }
}

/// `mcp_call` tool: call one tool exposed by a configured MCP server.
pub struct McpCallTool;

/// `mcp_list_tools` tool: list tools of one/a configured MCP server.
pub struct McpListToolsTool;

/// Base tool registered for role `tool.mcp`.
#[must_use]
pub fn tool() -> McpCallTool {
    McpCallTool
}

/// Extra tools contributed by this package (generated glue appends these).
#[must_use]
pub fn extra_tools() -> Vec<Arc<dyn Tool>> {
    vec![Arc::new(McpListToolsTool)]
}

impl Tool for McpCallTool {
    fn approval_metadata(
        &self,
        args: &serde_json::Value,
        context: &TurnContext,
    ) -> Option<ApprovalMetadata> {
        let server = args.get("server")?.as_str()?;
        let tool = args.get("tool")?.as_str()?;
        Some(ApprovalMetadata {
            summary: format!("call MCP tool {server}.{tool}"),
            cwd: context.workspace_root.clone(),
            network_targets: vec![format!("<mcp:{server}>")],
            risk: ApprovalRisk::High,
            ..ApprovalMetadata::default()
        })
    }
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "mcp_call".to_owned(),
            description: "Call a tool exposed by a configured MCP server (connector). Returns the raw MCP result JSON.".to_owned(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "server": {"type": "string", "description": "Configured MCP server id"},
                    "tool": {"type": "string", "description": "Tool name from mcp_list_tools"},
                    "arguments": {"type": "object", "description": "Tool arguments (JSON object)"}
                },
                "required": ["server", "tool"],
                "additionalProperties": false
            }),
            mutating: true,
            requires_approval: false,
        }
    }

    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "mcp_call requires JSON arguments".to_owned(),
        ))
    }

    fn call_json(&self, _context: &TurnContext, args: Value) -> Result<String, ToolError> {
        let server_id = args
            .get("server")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("mcp_call: server is required".to_owned()))?;
        let tool_name = args
            .get("tool")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::InvalidArgs("mcp_call: tool is required".to_owned()))?;
        let arguments = args.get("arguments").cloned().unwrap_or_else(|| json!({}));
        let server = find_server(server_id).map_err(tool_error)?;
        let result = call_server_tool(&server, tool_name, &arguments).map_err(tool_error)?;
        serde_json::to_string(&result)
            .map_err(|error| ToolError::Io(format!("mcp_call: encode result: {error}")))
    }
}

impl Tool for McpListToolsTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "mcp_list_tools".to_owned(),
            description:
                "List tools exposed by configured MCP servers (or one server id). Returns JSON."
                    .to_owned(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "server": {"type": "string", "description": "Optional MCP server id; omit to list all"}
                },
                "additionalProperties": false
            }),
            mutating: false,
            requires_approval: false,
        }
    }

    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "mcp_list_tools requires JSON arguments".to_owned(),
        ))
    }

    fn call_json(&self, _context: &TurnContext, args: Value) -> Result<String, ToolError> {
        let result = match args.get("server").and_then(Value::as_str) {
            Some(server_id) => {
                let server = find_server(server_id).map_err(tool_error)?;
                let listed = list_server_tools(&server).map_err(tool_error)?;
                json!({
                    "id": server.id,
                    "name": server.name,
                    "transport": server.transport,
                    "tools": listed.get("tools").cloned().unwrap_or_else(|| json!([])),
                })
            }
            None => list_all_servers().map_err(tool_error)?,
        };
        serde_json::to_string(&result)
            .map_err(|error| ToolError::Io(format!("mcp_list_tools: encode result: {error}")))
    }
}
