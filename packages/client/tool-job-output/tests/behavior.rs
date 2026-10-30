#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn reads_job_output() {
    let dir = std::env::temp_dir();
    let ctx = TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: dir.display().to_string() };
    let id = zk_kit_jobs::start(&ctx, "printf hi", None).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert!(zk_tool_job_output::tool().call_json(&ctx, serde_json::json!({"id":id})).unwrap().contains("hi"));
}
