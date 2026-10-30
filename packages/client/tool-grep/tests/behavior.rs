#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn finds_grep_matches() {
    let dir = std::env::temp_dir().join(format!(
        "zk-grep-{}",
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
    assert!(zk_tool_grep::tool()
        .call_json(&ctx, serde_json::json!({"pattern":"needle"}))
        .unwrap()
        .contains("a.txt:1"));
}
