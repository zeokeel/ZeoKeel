#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn schedule_crud() {
    let dir = std::env::temp_dir().join(format!("zk-schedule-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    let ctx = TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: dir.display().to_string() };
    let tool = zk_tool_schedule::tool();
    tool.call_json(&ctx, serde_json::json!({"action":"create","cron":"0 9 * * *","prompt":"standup"})).unwrap();
    assert!(tool.call_json(&ctx, serde_json::json!({"action":"list"})).unwrap().contains("standup"));
    tool.call_json(&ctx, serde_json::json!({"action":"delete","id":"schedule-1"})).unwrap();
}
