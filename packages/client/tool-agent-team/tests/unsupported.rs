#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
//! Isolated test binary: no runner is installed, so the team tool must degrade
//! honestly instead of pretending to dispatch workers.

use zk_contract_tool::{Tool, ToolError, TurnContext};

#[test]
fn missing_runner_is_an_explicit_error() {
    let context = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: ".".to_owned(),
    };
    let error = zk_tool_agent_team::tool()
        .call_json(
            &context,
            serde_json::json!({"objective": "x", "tasks": [{"prompt": "one"}]}),
        )
        .unwrap_err();
    assert!(
        matches!(error, ToolError::Denied(ref message) if message.contains("runner is not installed"))
    );
}
