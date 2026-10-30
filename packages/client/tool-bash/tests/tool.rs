#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};

#[test]
fn bash_json_requires_command() {
    let tool = zk_tool_bash::tool();
    let context = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: std::env::temp_dir().display().to_string(),
    };
    assert!(tool.call_json(&context, serde_json::json!({})).is_err());
    assert!(tool.spec().requires_approval);
}
