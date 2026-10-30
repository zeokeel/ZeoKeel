#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn presents_existing_file_only() {
    let dir = std::env::temp_dir().join(format!(
        "zk-present-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("out.txt"), "x").unwrap();
    let ctx = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: dir.display().to_string(),
    };
    let tool = zk_tool_present::tool();
    assert!(tool
        .call_json(
            &ctx,
            serde_json::json!({"path":"out.txt","description":"result"})
        )
        .unwrap()
        .contains("presented"));
    assert!(tool
        .call_json(&ctx, serde_json::json!({"path":"missing.txt"}))
        .is_err());
}
