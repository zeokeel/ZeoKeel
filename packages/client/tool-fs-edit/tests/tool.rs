#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};

#[test]
fn edit_requires_single_match() {
    let dir = std::env::temp_dir().join(format!(
        "zk-edit-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.txt"), "x x").unwrap();
    let context = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: dir.display().to_string(),
    };
    assert!(zk_tool_fs_edit::tool()
        .call_json(
            &context,
            serde_json::json!({"path":"a.txt","old_string":"x","new_string":"y"})
        )
        .is_err());
}
