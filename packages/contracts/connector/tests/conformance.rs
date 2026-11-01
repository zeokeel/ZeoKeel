//! Connector contract conformance: OAuth public-client metadata validation.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use zk_contract_connector::{
    is_loopback_uri, Connector, ConnectorCatalogItem, ConnectorOAuth, ConnectorTransport,
};

fn valid() -> ConnectorOAuth {
    ConnectorOAuth {
        client_id: "zk-public-client".to_owned(),
        scopes: vec!["mcp.read".to_owned(), "mcp.write".to_owned()],
        redirect_uri: String::new(),
        authorize_url: "https://auth.example.invalid/authorize".to_owned(),
        token_url: "https://auth.example.invalid/token".to_owned(),
    }
}

#[test]
fn valid_public_client_metadata_passes() {
    valid().validate().unwrap();

    // Loopback http is allowed for local dev / local MCP servers (RFC 8252).
    let loopback = ConnectorOAuth {
        authorize_url: "http://127.0.0.1:8080/authorize".to_owned(),
        token_url: "http://localhost/token".to_owned(),
        redirect_uri: "http://127.0.0.1:9999/callback".to_owned(),
        ..valid()
    };
    loopback.validate().unwrap();

    // Round-trips through serde (additive fields keep old JSON parseable).
    let text = serde_json::to_string(&loopback).unwrap();
    assert_eq!(
        loopback,
        serde_json::from_str::<ConnectorOAuth>(&text).unwrap()
    );
}

#[test]
fn legacy_oauth_json_without_endpoints_still_parses_but_fails_validation() {
    let legacy: ConnectorOAuth = serde_json::from_str(
        r#"{"client_id":"c","scopes":["a"],"redirect_uri":"http://127.0.0.1/cb"}"#,
    )
    .unwrap();
    assert_eq!(legacy.authorize_url, "");
    assert_eq!(legacy.token_url, "");
    assert!(legacy.validate().is_err(), "missing endpoints must fail");
}

#[test]
fn rejects_missing_or_malformed_public_client_fields() {
    let mut bad = valid();
    bad.client_id = "  ".to_owned();
    assert!(bad.validate().is_err());
    let mut bad = valid();
    bad.client_id = "a".repeat(257);
    assert!(bad.validate().is_err());
    let mut bad = valid();
    bad.client_id = "a\nb".to_owned();
    assert!(bad.validate().is_err());

    let mut bad = valid();
    bad.authorize_url.clear();
    assert!(bad.validate().is_err());
    let mut bad = valid();
    bad.token_url = "  ".to_owned();
    assert!(bad.validate().is_err());

    let mut bad = valid();
    bad.scopes = vec!["".to_owned()];
    assert!(bad.validate().is_err());
    let mut bad = valid();
    bad.scopes = vec!["mcp read".to_owned()];
    assert!(bad.validate().is_err());
}

#[test]
fn rejects_non_https_non_loopback_endpoints_and_userinfo() {
    for url in [
        "http://auth.example.invalid/authorize",
        "ftp://auth.example.invalid/authorize",
        "auth.example.invalid/authorize",
        "https://user:pass@auth.example.invalid/authorize",
        "https://auth.example.invalid/authorize#fragment",
        "https:///no-host",
    ] {
        let bad = ConnectorOAuth {
            authorize_url: url.to_owned(),
            ..valid()
        };
        assert!(
            bad.validate().is_err(),
            "authorize_url `{url}` must be rejected"
        );
        let bad = ConnectorOAuth {
            token_url: url.to_owned(),
            ..valid()
        };
        assert!(
            bad.validate().is_err(),
            "token_url `{url}` must be rejected"
        );
    }
}

#[test]
fn rejects_non_loopback_redirect_uri() {
    for redirect in [
        "https://app.example.invalid/callback",
        "myapp://oauth/callback",
        "http://example.invalid/cb",
    ] {
        let bad = ConnectorOAuth {
            redirect_uri: redirect.to_owned(),
            ..valid()
        };
        assert!(
            bad.validate().is_err(),
            "redirect_uri `{redirect}` must be rejected (loopback only)"
        );
    }
    // Empty redirect URI is the normal desktop case (client picks its loopback port).
    let empty = ConnectorOAuth {
        redirect_uri: String::new(),
        ..valid()
    };
    empty.validate().unwrap();
}

#[test]
fn loopback_detection_is_textual_and_port_insensitive() {
    for uri in [
        "http://127.0.0.1:1234/cb",
        "http://127.18.0.9/cb",
        "http://localhost/cb",
        "http://LOCALHOST:8080/cb",
        "http://[::1]:8080/cb",
    ] {
        assert!(is_loopback_uri(uri), "{uri} must be loopback");
    }
    for uri in [
        "https://127.0.0.1/cb",
        "http://localhost.example.com/cb",
        "http://10.0.0.1/cb",
        "http://[::2]/cb",
        "http://user@127.0.0.1/cb",
        "127.0.0.1/cb",
    ] {
        assert!(
            !is_loopback_uri(uri),
            "{uri} must not be treated as an http loopback URI"
        );
    }
}

#[test]
fn catalog_item_carries_oauth_metadata_additively() {
    let legacy: ConnectorCatalogItem = serde_json::from_str(
        r#"{"id":"c1","name":"Demo","transport":"http","enabled":true,"config":{"url":"https://mcp.example.invalid/mcp"}}"#,
    )
    .unwrap();
    assert!(legacy.oauth.is_none());

    let catalog: ConnectorCatalogItem = serde_json::from_str(
        &serde_json::json!({
            "id": "c1",
            "name": "Demo",
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
        })
        .to_string(),
    )
    .unwrap();
    let oauth = catalog.oauth.as_ref().expect("oauth metadata parsed");
    oauth.validate().unwrap();
    assert_eq!(oauth.client_id, "zk-public-client");
    assert_eq!(catalog.transport, ConnectorTransport::Http);
}

#[test]
fn connector_definition_json_is_unchanged() {
    // The aggregate `Connector` shape did not change with the OAuth additions.
    let connector: Connector = serde_json::from_str(
        r#"{"id":"c1","name":"Demo","transport":"stdio","auth_mode":"none","scopes":[],"enabled":true}"#,
    )
    .unwrap();
    assert_eq!(connector.transport, ConnectorTransport::Stdio);
}

#[test]
fn mcp_server_config_validate_rules_are_preserved() {
    use std::collections::BTreeMap;
    use zk_contract_connector::{ConnectorTransport, McpServerConfig};

    fn server() -> McpServerConfig {
        McpServerConfig {
            id: "mcp-demo".to_owned(),
            name: "demo".to_owned(),
            transport: ConnectorTransport::Stdio,
            command: "mcp-demo".to_owned(),
            args: Vec::new(),
            env: BTreeMap::new(),
            url: String::new(),
            timeout_ms: 0,
            oauth_required: false,
        }
    }

    let ok = server();
    ok.validate().unwrap();
    // id/name required.
    let mut bad = server();
    bad.id = " ".to_owned();
    assert!(bad.validate().is_err());
    // stdio requires command.
    let mut bad = server();
    bad.command = String::new();
    assert!(bad.validate().is_err());
    // secret-like env keys rejected (ADR-060).
    let mut bad = server();
    bad.env.insert("API_TOKEN".to_owned(), "x".to_owned());
    assert!(bad.validate().is_err());
    // http requires a URL and rejects plain http on non-loopback hosts.
    let mut bad = server();
    bad.transport = ConnectorTransport::Http;
    assert!(bad.validate().is_err());
    bad.url = "http://example.com/mcp".to_owned();
    assert!(bad.validate().is_err());
    bad.url = "http://127.0.0.1:8080/mcp".to_owned();
    bad.validate().unwrap();
    bad.url = "https://example.com/mcp".to_owned();
    bad.validate().unwrap();
    // Catalog rules do NOT apply local caps: a long name is still accepted.
    let mut long_name = server();
    long_name.name = "n".repeat(200);
    long_name.validate().unwrap();
}
