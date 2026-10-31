#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic
)] // test-only
//! stdio transport, catalog config mapping and session lifecycle tests (fake MCP server).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;
use zk_connector_mcp::{
    call_server_tool, clear_catalog_config, extra_tools, list_server_tools, parse_servers,
    reload_connectors, servers_from_catalog, update_from_catalog, validate_server, McpCallTool,
    SERVERS_ENV,
};
use zk_contract_connector::{ConnectorTransport, McpServerConfig};
use zk_contract_tool::{Tool, TurnContext};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn script() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("fake_mcp_server.py")
        .to_string_lossy()
        .into_owned()
}

fn fixture(mode: Option<&str>, timeout_ms: u64) -> McpServerConfig {
    let mut args = vec![script()];
    if let Some(mode) = mode {
        args.push(mode.to_owned());
    }
    McpServerConfig {
        id: "fake".to_owned(),
        name: "Fake MCP".to_owned(),
        transport: ConnectorTransport::Stdio,
        command: "python3".to_owned(),
        args,
        env: std::collections::BTreeMap::new(),
        url: String::new(),
        timeout_ms,
        oauth_required: false,
    }
}

fn countfile_path() -> PathBuf {
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("zk-mcp-count-{}-{unique}.txt", std::process::id()))
}

fn countfile_fixture(path: &Path) -> McpServerConfig {
    McpServerConfig {
        id: "countfile".to_owned(),
        name: "Countfile MCP".to_owned(),
        transport: ConnectorTransport::Stdio,
        command: "python3".to_owned(),
        args: vec![
            script(),
            "countfile".to_owned(),
            path.to_string_lossy().into_owned(),
        ],
        env: std::collections::BTreeMap::new(),
        url: String::new(),
        timeout_ms: 5_000,
        oauth_required: false,
    }
}

fn initialize_count(path: &Path) -> u64 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| text.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

fn python_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn http_server(url: &str, timeout_ms: u64) -> McpServerConfig {
    McpServerConfig {
        id: "http".to_owned(),
        name: "HTTP MCP".to_owned(),
        transport: ConnectorTransport::Http,
        command: String::new(),
        args: Vec::new(),
        env: std::collections::BTreeMap::new(),
        url: url.to_owned(),
        timeout_ms,
        oauth_required: false,
    }
}

#[test]
fn lists_tools_from_stdio_server() {
    if !python_available() {
        eprintln!("skip: python3 not available");
        return;
    }
    let result = list_server_tools(&fixture(None, 5_000)).unwrap();
    let tools = result
        .get("tools")
        .and_then(serde_json::Value::as_array)
        .unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(
        tools[0].get("name").and_then(serde_json::Value::as_str),
        Some("echo")
    );
}

#[test]
fn calls_tool_from_stdio_server() {
    if !python_available() {
        eprintln!("skip: python3 not available");
        return;
    }
    let result =
        call_server_tool(&fixture(None, 5_000), "echo", &json!({"text": "hello"})).unwrap();
    let content = result
        .get("content")
        .and_then(serde_json::Value::as_array)
        .unwrap();
    let text = content[0]
        .get("text")
        .and_then(serde_json::Value::as_str)
        .unwrap();
    assert!(text.contains("hello"), "{text}");
    assert!(text.contains("echo"), "{text}");
}

#[test]
fn timeout_is_reported() {
    if !python_available() {
        eprintln!("skip: python3 not available");
        return;
    }
    let error = list_server_tools(&fixture(Some("timeout"), 300)).unwrap_err();
    assert!(error.starts_with("MCP_TIMEOUT"), "{error}");
}

#[test]
fn malformed_jsonrpc_is_reported() {
    if !python_available() {
        eprintln!("skip: python3 not available");
        return;
    }
    let error = list_server_tools(&fixture(Some("malformed"), 5_000)).unwrap_err();
    assert!(error.starts_with("MCP_PROTOCOL"), "{error}");
}

#[test]
fn nonzero_exit_is_reported() {
    if !python_available() {
        eprintln!("skip: python3 not available");
        return;
    }
    let error = list_server_tools(&fixture(Some("exit"), 5_000)).unwrap_err();
    assert!(error.starts_with("MCP_EOF"), "{error}");
}

#[test]
fn config_validation_rejects_secrets_and_insecure_transports() {
    let mut secret = fixture(None, 1_000);
    secret
        .env
        .insert("FAKE_API_TOKEN".to_owned(), "value".to_owned());
    let error = validate_server(&secret).unwrap_err();
    assert!(error.starts_with("MCP_VALIDATION"), "{error}");

    let mut no_command = fixture(None, 1_000);
    no_command.command.clear();
    assert!(validate_server(&no_command)
        .unwrap_err()
        .starts_with("MCP_VALIDATION"));

    // Plain http is loopback/dev-only; https is accepted (TLS client implemented).
    let insecure = http_server("http://example.com/mcp", 1_000);
    assert!(validate_server(&insecure)
        .unwrap_err()
        .starts_with("MCP_INSECURE_TRANSPORT"));
    let secure = http_server("https://example.com/mcp", 1_000);
    assert!(validate_server(&secure).is_ok());

    let mut bad_json = parse_servers("not-json").unwrap_err();
    bad_json.truncate(64);
    assert!(bad_json.starts_with("MCP_CONFIG"));

    // URL whitespace must not reach the HTTP request line.
    let spaced = http_server("http://127.0.0.1:1/ x", 1_000);
    assert!(validate_server(&spaced)
        .unwrap_err()
        .starts_with("MCP_VALIDATION"));
}

#[test]
fn sessions_reuse_initialize_and_force_update_restarts() {
    if !python_available() {
        eprintln!("skip: python3 not available");
        return;
    }
    clear_catalog_config().unwrap();
    std::env::remove_var(SERVERS_ENV);
    let path = countfile_path();
    let config = countfile_fixture(&path);
    // Same config: one initialize, reused for both calls.
    list_server_tools(&config).unwrap();
    list_server_tools(&config).unwrap();
    assert_eq!(initialize_count(&path), 1);
    // Explicit reload (force-update hook) drops the session => re-initialize.
    reload_connectors().unwrap();
    list_server_tools(&config).unwrap();
    assert_eq!(initialize_count(&path), 2);
    // Config fingerprint change restarts the session automatically.
    let mut changed = config.clone();
    changed.timeout_ms = 4_000;
    list_server_tools(&changed).unwrap();
    assert_eq!(initialize_count(&path), 3);

    // Catalog config passthrough: enabled stdio item maps to MCP server and is used by
    // the agent tools; second catalog refresh acts as force-update.
    let catalog = |timeout_ms: u64| {
        json!({"items": [{
            "id": "cat-stdio",
            "name": "Catalog MCP",
            "transport": "stdio",
            "enabled": true,
            "config": {
                "command": "python3",
                "args": [script(), "countfile", path.to_string_lossy()],
                "timeout_ms": timeout_ms
            }
        }]})
        .to_string()
    };
    let mapped = servers_from_catalog(&catalog(5_000)).unwrap();
    assert_eq!(mapped.len(), 1);
    assert_eq!(mapped[0].command, "python3");
    assert_eq!(update_from_catalog(&catalog(5_000)).unwrap(), 1);

    let list_tool = zk_connector_mcp::extra_tools();
    let context = TurnContext {
        conversation_id: String::new(),
        workspace_id: String::new(),
        workspace_root: String::new(),
    };
    let before = initialize_count(&path);
    list_tool[0].call_json(&context, json!({})).unwrap();
    list_tool[0].call_json(&context, json!({})).unwrap();
    assert_eq!(initialize_count(&path), before.saturating_add(1));
    // Force-update with changed config => session restart on next call.
    update_from_catalog(&catalog(4_000)).unwrap();
    list_tool[0].call_json(&context, json!({})).unwrap();
    assert_eq!(initialize_count(&path), before.saturating_add(2));
    clear_catalog_config().unwrap();

    // Rejection: secret-looking env key in catalog config.
    let secret_catalog = json!({"items": [{
        "id": "cat-secret",
        "name": "Secret MCP",
        "transport": "stdio",
        "enabled": true,
        "config": {"command": "python3", "env": {"MCP_TOKEN": "x"}}
    }]})
    .to_string();
    assert!(servers_from_catalog(&secret_catalog)
        .unwrap_err()
        .starts_with("MCP_VALIDATION"));
    // Disabled connectors are not exposed.
    let disabled_catalog = json!({"items": [{
        "id": "cat-disabled",
        "name": "Disabled",
        "transport": "stdio",
        "enabled": false,
        "config": {"command": "python3"}
    }]})
    .to_string();
    assert!(servers_from_catalog(&disabled_catalog).unwrap().is_empty());
}

#[test]
fn tools_are_registered() {
    let call = McpCallTool;
    assert_eq!(call.spec().name, "mcp_call");
    assert_eq!(
        call.spec()
            .parameters
            .get("type")
            .and_then(serde_json::Value::as_str),
        Some("object")
    );
    assert_eq!(extra_tools().len(), 1);
    assert_eq!(extra_tools()[0].spec().name, "mcp_list_tools");
}
