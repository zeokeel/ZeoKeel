#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::sync::{Mutex, MutexGuard, OnceLock};
use zk_contract_config::{RuntimeSettings, WebSearchSettings};
use zk_contract_tool::{Tool, TurnContext};

/// Env vars and the published runtime snapshot are process-global; serialize the
/// tests that mutate them (the Rust test harness runs tests in parallel threads).
fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn publish(web_search: WebSearchSettings) {
    zk_contract_config::set_runtime_settings(&RuntimeSettings {
        web_search: Some(web_search),
        ..RuntimeSettings::default()
    })
    .unwrap();
}

#[test]
fn rejects_non_https_env_endpoint() {
    let _guard = env_lock();
    zk_contract_config::clear_runtime_settings().unwrap();
    std::env::set_var("ZEOKEEL_SEARCH_URL", "http://127.0.0.1/");
    let ctx = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: ".".to_owned(),
    };
    assert!(zk_tool_web_search::tool()
        .call_json(&ctx, serde_json::json!({"query":"x"}))
        .is_err());
    std::env::remove_var("ZEOKEEL_SEARCH_URL");
}

#[test]
fn resolves_settings_then_env_then_default() {
    let _guard = env_lock();
    zk_contract_config::clear_runtime_settings().unwrap();
    std::env::remove_var("ZEOKEEL_SEARCH_URL");

    // Fresh install: no publish, no env -> built-in default.
    let endpoint = zk_tool_web_search::resolve_endpoint();
    assert_eq!(endpoint.base_url, zk_tool_web_search::DEFAULT_SEARCH_URL);
    assert_eq!(endpoint.timeout_ms, zk_tool_web_search::DEFAULT_TIMEOUT_MS);

    // Env fallback (direct embedding / fresh install) wins over the default.
    std::env::set_var("ZEOKEEL_SEARCH_URL", "https://env.example.invalid/api");
    let endpoint = zk_tool_web_search::resolve_endpoint();
    assert_eq!(endpoint.base_url, "https://env.example.invalid/api");
    assert_eq!(endpoint.timeout_ms, zk_tool_web_search::DEFAULT_TIMEOUT_MS);

    // Explicit settings win over the env fallback, including the timeout.
    publish(WebSearchSettings {
        base_url: "https://settings.example.invalid/api".to_owned(),
        timeout_ms: 30_000,
    });
    let endpoint = zk_tool_web_search::resolve_endpoint();
    assert_eq!(endpoint.base_url, "https://settings.example.invalid/api");
    assert_eq!(endpoint.timeout_ms, 30_000);

    // A blank published base_url is "no override": env stays authoritative while
    // the explicit timeout still applies.
    publish(WebSearchSettings {
        base_url: String::new(),
        timeout_ms: 45_000,
    });
    let endpoint = zk_tool_web_search::resolve_endpoint();
    assert_eq!(endpoint.base_url, "https://env.example.invalid/api");
    assert_eq!(endpoint.timeout_ms, 45_000);

    // Out-of-range timeouts are rejected at publish time (contract validation),
    // so the tool never has to clamp a snapshot it reads back.
    assert!(zk_contract_config::set_runtime_settings(&RuntimeSettings {
        web_search: Some(WebSearchSettings {
            base_url: String::new(),
            timeout_ms: u64::MAX,
        }),
        ..RuntimeSettings::default()
    })
    .is_err());

    std::env::remove_var("ZEOKEEL_SEARCH_URL");
    zk_contract_config::clear_runtime_settings().unwrap();
}
