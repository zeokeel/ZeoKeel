#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn disabled_team_is_explicit() {
    let ctx = TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: ".".to_owned() };
    assert!(zk_tool_agent_team::tool().call_json(&ctx, serde_json::json!({})).is_err());
}
