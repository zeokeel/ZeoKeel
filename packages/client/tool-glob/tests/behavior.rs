#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn finds_glob_matches() {
    let dir = std::env::temp_dir().join(format!(
        "zk-glob-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/a.rs"), "").unwrap();
    let ctx = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: dir.display().to_string(),
    };
    assert!(zk_tool_glob::tool()
        .call_json(&ctx, serde_json::json!({"pattern":"**/*.rs"}))
        .unwrap()
        .contains("src/a.rs"));
}
