#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn lists_jobs() {
    let ctx = TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: std::env::temp_dir().display().to_string() };
    zk_kit_jobs::start(&ctx, "true", None).unwrap();
    assert!(zk_tool_job_list::tool().call_json(&ctx, serde_json::json!({})).unwrap().contains("job-"));
}
