#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic
)] // test-only
//! HTTP(S) transport tests against in-process TCP fixtures (no external network).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use serde_json::{json, Value};
use zk_connector_mcp::{list_server_tools, validate_server};
use zk_contract_connector::{ConnectorTransport, McpServerConfig};

fn http_config(url: &str) -> McpServerConfig {
    McpServerConfig {
        id: "http".to_owned(),
        name: "HTTP MCP".to_owned(),
        transport: ConnectorTransport::Http,
        command: String::new(),
        args: Vec::new(),
        env: std::collections::BTreeMap::new(),
        url: url.to_owned(),
        timeout_ms: 5_000,
        oauth_required: false,
    }
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
            "serverInfo": {"name": "fake-http", "version": "0.0.1"},
        }),
        "tools/list" => json!({
            "tools": [{"name": "echo", "description": "Echo", "inputSchema": {"type": "object"}}],
        }),
        "tools/call" => json!({
            "content": [{"type": "text", "text": "{\"tool\":\"echo\"}"}],
            "isError": false,
        }),
        _ => json!({}),
    }
}

fn spawn_http_server(
    mode: &'static str,
    expected_requests: usize,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let url = format!("http://{}:{}/mcp", address.ip(), address.port());
    let handle = thread::spawn(move || {
        for _ in 0..expected_requests {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();
            let value: Value = serde_json::from_str(body).unwrap();
            let id = value.get("id").and_then(Value::as_u64).unwrap();
            let method = value
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let payload =
                json!({"jsonrpc": "2.0", "id": id, "result": rpc_result(method)}).to_string();
            let (content_type, body) = if mode == "sse" {
                ("text/event-stream", format!("data: {payload}\n\n"))
            } else {
                ("application/json", payload)
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.flush().unwrap();
        }
    });
    (url, handle)
}

/// Accept one TCP connection and return the first bytes the client wrote (TLS probe).
fn spawn_tls_probe() -> (String, thread::JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let url = format!("https://{}:{}/mcp", address.ip(), address.port());
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        let mut buffer = [0_u8; 64];
        let read = stream.read(&mut buffer).unwrap_or(0);
        buffer[..read].to_vec()
    });
    (url, handle)
}

#[test]
fn http_json_post_lists_tools() {
    let (url, handle) = spawn_http_server("json", 2);
    let result = list_server_tools(&http_config(&url)).unwrap();
    handle.join().unwrap();
    let tools = result.get("tools").and_then(Value::as_array).unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].get("name").and_then(Value::as_str), Some("echo"));
}

#[test]
fn http_sse_response_lists_tools() {
    let (url, handle) = spawn_http_server("sse", 2);
    let result = list_server_tools(&http_config(&url)).unwrap();
    handle.join().unwrap();
    let tools = result.get("tools").and_then(Value::as_array).unwrap();
    assert_eq!(tools.len(), 1);
}

#[test]
fn https_uses_tls_client_hello_and_fails_honestly() {
    let (url, handle) = spawn_tls_probe();
    let error = list_server_tools(&http_config(&url)).unwrap_err();
    let hello = handle.join().unwrap();
    assert!(error.starts_with("MCP_HTTP"), "{error}");
    assert!(!hello.is_empty(), "client did not connect");
    assert_eq!(hello[0], 0x16, "expected TLS ClientHello, got {hello:?}");
}

#[test]
fn https_is_accepted_by_validation() {
    let config = http_config("https://example.com/mcp");
    assert!(validate_server(&config).is_ok());
}
