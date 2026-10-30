#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::sync::Arc;
use zk_contract_tool::{Tool, TurnContext};
struct R;
impl zk_kit_subagent::SubagentRunner for R {
    fn run(&self, prompt: &str, _c: &TurnContext, _d: usize) -> Result<String, String> {
        Ok(prompt.to_owned())
    }
}
#[test]
fn ralph_bounded() {
    zk_kit_subagent::set_runner(Arc::new(R));
    let ctx = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: ".".to_owned(),
    };
    let out = zk_tool_ralph::tool()
        .call_json(&ctx, serde_json::json!({"objective":"x","max_rounds":2}))
        .unwrap();
    assert!(out.contains("rounds") && out.contains("round 2"));
}
