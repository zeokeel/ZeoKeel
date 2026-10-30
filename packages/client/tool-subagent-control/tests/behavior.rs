#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn lists_registered_children() {
    zk_kit_subagent::register("subagent-x", "parent", 1, "child task");
    zk_kit_subagent::finish("subagent-x", true);
    let ctx = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: ".".to_owned(),
    };
    let out = zk_tool_subagent_control::tool()
        .call_json(&ctx, serde_json::json!({}))
        .unwrap();
    assert!(out.contains("subagent-x") && out.contains("done"));
}
