#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn searches_content() {
    let dir = std::env::temp_dir().join(format!(
        "zk-fss-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.txt"), "needle").unwrap();
    let ctx = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: dir.display().to_string(),
    };
    let out = zk_tool_fs_search::tool()
        .call_json(&ctx, serde_json::json!({"query":"needle"}))
        .unwrap();
    assert!(out.contains("a.txt"));
}
