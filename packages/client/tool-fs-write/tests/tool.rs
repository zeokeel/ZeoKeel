#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};

#[test]
fn write_rejects_outside_workspace() {
    let dir = std::env::temp_dir().join(format!(
        "zk-write-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let context = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: dir.display().to_string(),
    };
    assert!(zk_tool_fs_write::tool()
        .call_json(&context, serde_json::json!({"path":"../x","content":"x"}))
        .is_err());
}

#[test]
fn approval_metadata_reports_path_diff_and_risk() {
    let dir = std::env::temp_dir().join(format!(
        "zk-write-meta-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join("notes")).unwrap();
    std::fs::write(dir.join("notes/a.txt"), "one\ntwo\n").unwrap();
    let context = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: dir.display().to_string(),
    };
    let metadata = zk_tool_fs_write::tool()
        .approval_metadata(
            &serde_json::json!({"path":"notes/a.txt","content":"one\nthree\n"}),
            &context,
        )
        .expect("metadata");
    assert_eq!(metadata.write_paths, vec!["notes/a.txt".to_owned()]);
    assert_eq!(metadata.cwd, dir.display().to_string());
    assert!(metadata.diff.unwrap().contains("+three"));
    assert_eq!(
        zk_tool_fs_write::tool().approval_metadata(
            &serde_json::json!({"path":"../outside.txt","content":"x"}),
            &context
        ),
        None
    );
}
