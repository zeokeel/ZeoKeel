//! `zk-server`: minimal axum skeleton.
//!
//! The server is the thin management companion (ADR-028): it only exposes
//! health and metadata endpoints for the composed build. **It never runs an
//! agent**; the agent loop is client-side.
#![forbid(unsafe_code)]

use axum::{routing::get, Json, Router};
use serde_json::{json, Value};

/// Default listen address when `ZEOKEEL_SERVER_ADDR` is unset.
const DEFAULT_ADDR: &str = "127.0.0.1:8787";

fn health_payload() -> Value {
    json!({
        "status": "ok",
        "edition": zk_composed::EDITION,
        "packages": zk_composed::PACKAGES,
    })
}

fn ready_payload() -> Value {
    json!({ "status": "ready" })
}

fn meta_payload() -> Value {
    json!({
        "status": "ok",
        "edition": zk_composed::EDITION,
        "packages": zk_composed::PACKAGES,
        "version": env!("CARGO_PKG_VERSION"),
    })
}

async fn healthz() -> Json<Value> {
    Json(health_payload())
}

async fn readyz() -> Json<Value> {
    Json(ready_payload())
}

async fn api_meta() -> Json<Value> {
    Json(meta_payload())
}

fn app() -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/api/meta", get(api_meta))
}

fn server_addr() -> String {
    std::env::var("ZEOKEEL_SERVER_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_owned())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let addr = server_addr();
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|err| format!("failed to bind {addr}: {err}"))?;

    println!("zk-server listening on http://{addr}");
    axum::serve(listener, app())
        .await
        .map_err(|err| format!("zk-server failed: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edition_is_composed_and_non_empty() {
        assert!(
            !zk_composed::EDITION.is_empty(),
            "composed edition must not be empty"
        );
    }

    #[test]
    fn health_payload_shape() {
        let payload = health_payload();
        assert_eq!(payload.get("status").and_then(Value::as_str), Some("ok"));
        assert_eq!(
            payload.get("edition").and_then(Value::as_str),
            Some(zk_composed::EDITION)
        );
        assert!(payload.get("packages").is_some_and(Value::is_array));
    }

    #[test]
    fn ready_payload_shape() {
        let payload = ready_payload();
        assert_eq!(payload.get("status").and_then(Value::as_str), Some("ready"));
    }

    #[test]
    fn meta_payload_has_version() {
        let payload = meta_payload();
        assert_eq!(
            payload.get("version").and_then(Value::as_str),
            Some(env!("CARGO_PKG_VERSION"))
        );
        assert!(payload.get("packages").is_some_and(Value::is_array));
    }

    #[test]
    fn default_addr_is_local_only() {
        assert_eq!(DEFAULT_ADDR, "127.0.0.1:8787");
    }
}
