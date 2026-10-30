#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn rejects_non_https_endpoint() {
    std::env::set_var("ZEOKEEL_SEARCH_URL", "http://127.0.0.1/");
    let ctx = TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: ".".to_owned() };
    assert!(zk_tool_web_search::tool().call_json(&ctx, serde_json::json!({"query":"x"})).is_err());
    std::env::remove_var("ZEOKEEL_SEARCH_URL");
}
