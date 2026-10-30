#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn kills_job() {
    let ctx = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: std::env::temp_dir().display().to_string(),
    };
    let id = zk_kit_jobs::start(&ctx, "sleep 5", None).unwrap();
    assert!(zk_tool_job_kill::tool()
        .call_json(&ctx, serde_json::json!({"id":id}))
        .unwrap()
        .contains("killed"));
}
