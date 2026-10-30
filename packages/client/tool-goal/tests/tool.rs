#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn goal_roundtrip() {
    let dir = std::env::temp_dir().join(format!("zk-goal-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    let ctx = TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: dir.display().to_string() };
    assert_eq!(zk_tool_goal::tool().call_json(&ctx, serde_json::json!({"action":"get"})).unwrap(), "null");
    zk_tool_goal::tool().call_json(&ctx, serde_json::json!({"action":"create","objective":"ship"})).unwrap();
    let updated = zk_tool_goal::tool().call_json(&ctx, serde_json::json!({"action":"update","status":"complete"})).unwrap();
    assert!(updated.contains("complete"));
}
