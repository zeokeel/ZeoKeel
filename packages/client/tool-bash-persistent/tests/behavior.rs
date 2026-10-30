#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn persistent_cwd() {
    let dir = std::env::temp_dir().join(format!(
        "zk-bp-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    let ctx = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: dir.display().to_string(),
    };
    let tool = zk_tool_bash_persistent::tool();
    tool.call_json(&ctx, serde_json::json!({"command":"cd sub"}))
        .unwrap();
    assert!(tool
        .call_json(&ctx, serde_json::json!({"command":"pwd"}))
        .unwrap()
        .contains("sub"));
    zk_kit_tools::persistent::close_all();
}
