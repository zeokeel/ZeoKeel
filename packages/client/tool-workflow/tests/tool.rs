#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::sync::Arc;
use zk_contract_tool::{Tool, TurnContext};
struct R;
impl zk_kit_subagent::SubagentRunner for R {
    fn run(&self, prompt: &str, _c: &TurnContext, _d: usize) -> Result<String, String> { Ok(format!("done:{prompt}")) }
}
#[test]
fn workflow_runs_steps() {
    zk_kit_subagent::set_runner(Arc::new(R));
    let ctx = TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: ".".to_owned() };
    let out = zk_tool_workflow::tool().call_json(&ctx, serde_json::json!({"steps":[{"prompt":"a"},{"prompt":"b"}]})).unwrap();
    assert!(out.contains("done:a") && out.contains("done:b"));
}
