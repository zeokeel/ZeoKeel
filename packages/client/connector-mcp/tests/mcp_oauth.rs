#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic
)] // test-only
//! OAuth bearer-token injection for the HTTP MCP transport (docs/07 §6).
//!
//! Local fixture only: asserts the `Authorization: Bearer ...` header is attached
//! after [`set_oauth_token`], absent/cleared afterwards, and that an OAuth-gated
//! server fails honestly with `MCP_OAUTH_REQUIRED` when no token is installed.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread;

use serde_json::{json, Value};
use zk_connector_mcp::{
    clear_catalog_config, clear_oauth_tokens, has_oauth_token, list_server_tools, parse_servers,
    servers_from_catalog, set_oauth_token,
};
use zk_contract_connector::{ConnectorTransport, McpServerConfig};

/// Process-global token/catalog/session state: serialize the tests in this binary.
fn test_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn headers_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn content_length(head: &str) -> usize {
    head.lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            if key.trim().eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let read = stream.read(&mut chunk).unwrap();
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(position) = headers_end(&buffer) {
            let head = String::from_utf8_lossy(&buffer[..position]).into_owned();
            let expected = content_length(&head);
            let have = buffer.len() - (position + 4);
            if have >= expected {
                break;
            }
        }
    }
    String::from_utf8_lossy(&buffer).into_owned()
}

fn rpc_result(method: &str) -> Value {
    match method {
        "initialize" => json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "fake-oauth-http", "version": "0.0.1"},
        }),
        "tools/list" => json!({
            "tools": [{"name": "echo", "description": "Echo", "inputSchema": {"type": "object"}}],
        }),
        _ => json!({}),
    }
}

/// Spawn a local MCP HTTP fixture that records `Authorization` header values.
fn spawn_http_server(
    expected_requests: usize,
) -> (String, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let url = format!("http://{}:{}/mcp", address.ip(), address.port());
    let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let thread_seen = Arc::clone(&seen);
    let handle = thread::spawn(move || {
        for _ in 0..expected_requests {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let request = read_request(&mut stream);
            let mut lines = request.split("\r\n");
            let head = lines.next().unwrap_or_default();
            assert!(head.starts_with("POST "), "{head}");
            let authorization = request
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    if key.trim().eq_ignore_ascii_case("authorization") {
                        Some(value.trim().to_owned())
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            thread_seen.lock().unwrap().push(authorization);
            let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();
            let value: Value = serde_json::from_str(body).unwrap();
            let id = value.get("id").and_then(Value::as_u64).unwrap();
            let method = value
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let payload =
                json!({"jsonrpc": "2.0", "id": id, "result": rpc_result(method)}).to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                payload.len(),
                body = payload
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    (url, seen, handle)
}

fn oauth_config(url: &str, oauth_required: bool) -> McpServerConfig {
    McpServerConfig {
        id: "oauth-http".to_owned(),
        name: "OAuth HTTP MCP".to_owned(),
        transport: ConnectorTransport::Http,
        command: String::new(),
        args: Vec::new(),
        env: std::collections::BTreeMap::new(),
        url: url.to_owned(),
        timeout_ms: 5_000,
        oauth_required,
    }
}

#[test]
fn oauth_token_is_injected_only_after_set_and_cleared_after_clear() {
    let _guard = test_lock();
    // State is process-global: keep this test single-threaded over the token map.
    let _ = clear_oauth_tokens();
    let (url, seen, handle) = spawn_http_server(2);
    let server = oauth_config(&url, true);

    // No token yet: honest failure before any network call.
    let error = list_server_tools(&server).unwrap_err();
    assert!(error.starts_with("MCP_OAUTH_REQUIRED"), "{error}");
    assert!(
        !error.contains("Bearer"),
        "no header material in errors: {error}"
    );
    assert!(!has_oauth_token("oauth-http"));

    // Token installed: request carries `Authorization: Bearer <token>`.
    set_oauth_token("oauth-http", Some("token-super-secret".to_owned())).unwrap();
    assert!(has_oauth_token("oauth-http"));
    let result = list_server_tools(&server).unwrap();
    handle.join().unwrap();
    assert_eq!(
        result.get("tools").and_then(Value::as_array).map(Vec::len),
        Some(1)
    );
    let headers = seen.lock().unwrap().clone();
    assert_eq!(headers.len(), 2, "initialize + tools/list");
    assert!(
        headers
            .iter()
            .all(|header| header == "Bearer token-super-secret"),
        "{headers:?}"
    );

    // Cleared: gated again, and the token value is gone.
    set_oauth_token("oauth-http", None).unwrap();
    assert!(!has_oauth_token("oauth-http"));
    let error = list_server_tools(&server).unwrap_err();
    assert!(error.starts_with("MCP_OAUTH_REQUIRED"), "{error}");
    let _ = clear_oauth_tokens();
}

#[test]
fn non_oauth_servers_do_not_send_authorization() {
    let _guard = test_lock();
    let _ = clear_oauth_tokens();
    let (url, seen, handle) = spawn_http_server(2);
    let server = oauth_config(&url, false);
    set_oauth_token("oauth-http", Some("token-unused".to_owned())).unwrap();
    list_server_tools(&server).unwrap();
    handle.join().unwrap();
    let headers = seen.lock().unwrap().clone();
    assert_eq!(headers.len(), 2);
    assert!(headers.iter().all(String::is_empty), "{headers:?}");
    let _ = clear_oauth_tokens();
}

#[test]
fn token_values_are_validated_and_never_echoed() {
    let _guard = test_lock();
    let _ = clear_oauth_tokens();
    for (server_id, token) in [
        ("", Some("t".to_owned())),
        ("../evil", Some("t".to_owned())),
        ("ok", Some(String::new())),
        ("ok", Some("has space".to_owned())),
        ("ok", Some("line\nbreak".to_owned())),
        ("ok", Some("x".repeat(9_000))),
    ] {
        let error = set_oauth_token(server_id, token).expect_err("must be rejected");
        assert!(error.starts_with("MCP_VALIDATION"), "{error}");
        assert!(!error.contains("has space"), "{error}");
        assert!(!error.contains("line"), "{error}");
    }
    // `None` clears a never-installed id without error.
    set_oauth_token("ok", None).unwrap();
    assert!(!has_oauth_token("ok"));
}

#[test]
fn catalog_metadata_sets_oauth_required_and_force_update_retains_only_live_tokens() {
    let _guard = test_lock();
    let _ = clear_catalog_config();
    let _ = clear_oauth_tokens();
    let catalog = |ids: &[&str]| {
        json!({
            "items": ids.iter().map(|id| json!({
                "id": id,
                "name": "OAuth MCP",
                "transport": "http",
                "enabled": true,
                "config": {"url": "https://mcp.example.invalid/mcp"},
                "oauth": {
                    "client_id": "zk-public-client",
                    "scopes": ["mcp.read"],
                    "redirect_uri": "",
                    "authorize_url": "https://auth.example.invalid/authorize",
                    "token_url": "https://auth.example.invalid/token"
                }
            })).collect::<Vec<_>>()
        })
        .to_string()
    };

    // OAuth metadata maps to `oauth_required` on the client server config.
    let servers = servers_from_catalog(&catalog(&["a"])).unwrap();
    assert_eq!(servers.len(), 1);
    assert!(servers[0].oauth_required);
    // Local env config without oauth metadata stays non-OAuth.
    let local = parse_servers(
        r#"[{"id":"b","name":"Local","transport":"http","url":"https://mcp.example.invalid/mcp"}]"#,
    )
    .unwrap();
    assert!(!local[0].oauth_required);

    set_oauth_token("a", Some("token-a".to_owned())).unwrap();
    set_oauth_token("stale", Some("token-stale".to_owned())).unwrap();
    // Force update keeps tokens for live connectors only.
    zk_connector_mcp::update_from_catalog(&catalog(&["a"])).unwrap();
    assert!(has_oauth_token("a"));
    assert!(!has_oauth_token("stale"));
    // Clearing the catalog config (logout/disconnect path) drops all tokens.
    clear_catalog_config().unwrap();
    assert!(!has_oauth_token("a"));
    let _ = clear_oauth_tokens();
}
