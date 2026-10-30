#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::sync::Arc;
use zk_contract_tool::{Tool, TurnContext};
struct R;
impl zk_kit_subagent::SubagentRunner for R {
    fn run(&self, p: &str, _c: &TurnContext, _d: usize) -> Result<String, String> {
        Ok(p.to_owned())
    }
}
#[test]
fn forks_with_context() {
    zk_kit_subagent::set_runner(Arc::new(R));
    let ctx = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: ".".to_owned(),
    };
    assert!(zk_tool_subagent_fork::tool()
        .call_json(&ctx, serde_json::json!({"prompt":"fork"}))
        .unwrap()
        .contains("fork"));
}
