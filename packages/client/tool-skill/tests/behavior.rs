#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use zk_contract_tool::{Tool, TurnContext};
#[test]
fn lists_and_reads_skills() {
    let dir = std::env::temp_dir().join(format!("zk-skill-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(dir.join(".agents/skills/demo")).unwrap(); std::fs::write(dir.join(".agents/skills/demo/SKILL.md"), "body").unwrap();
    let ctx = TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: dir.display().to_string() };
    let tool = zk_tool_skill::tool();
    assert!(tool.call_json(&ctx, serde_json::json!({"action":"list"})).unwrap().contains("demo"));
    assert_eq!(tool.call_json(&ctx, serde_json::json!({"action":"read","name":"demo"})).unwrap(), "body");
}
