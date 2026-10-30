#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::sync::Arc;
use zk_contract_tool::{Tool, TurnContext};
struct R;
impl zk_kit_subagent::SubagentRunner for R { fn run(&self, prompt:&str,_c:&TurnContext,_d:usize)->Result<String,String>{Ok(prompt.to_owned())} }
fn ctx() -> TurnContext { TurnContext { conversation_id: "c".to_owned(), workspace_id: "w".to_owned(), workspace_root: ".".to_owned() } }
#[test]
fn dag_runs_dependencies_first() {
    zk_kit_subagent::set_runner(Arc::new(R));
    let out = zk_tool_workflow::tool().call_json(&ctx(), serde_json::json!({"steps":[{"id":"b","prompt":"second","depends_on":["a"]},{"id":"a","prompt":"first"}]})).unwrap();
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    let steps = value.get("steps").and_then(serde_json::Value::as_array).unwrap();
    assert_eq!(steps.first().and_then(|s| s.get("id")).and_then(serde_json::Value::as_str), Some("a"));
}
#[test]
fn cycle_is_rejected() {
    zk_kit_subagent::set_runner(Arc::new(R));
    assert!(zk_tool_workflow::tool().call_json(&ctx(), serde_json::json!({"steps":[{"id":"a","prompt":"a","depends_on":["b"]},{"id":"b","prompt":"b","depends_on":["a"]}]})).is_err());
}
