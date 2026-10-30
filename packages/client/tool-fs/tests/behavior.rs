#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn reads_text_and_rejects_escape() {
    let dir = std::env::temp_dir().join(format!(
        "zk-fs-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.txt"), "hello").unwrap();
    let ctx = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: dir.display().to_string(),
    };
    let tool = zk_tool_fs::tool();
    assert_eq!(
        tool.call_json(&ctx, serde_json::json!({"path":"a.txt"}))
            .unwrap(),
        "hello"
    );
    assert!(tool
        .call_json(&ctx, serde_json::json!({"path":"../x"}))
        .is_err());
}
