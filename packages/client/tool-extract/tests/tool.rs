#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::sync::Arc;
use zk_contract_tool::{Tool, TurnContext};
struct R;
impl zk_kit_extractor::ExtractRunner for R {
    fn extract(&self, text: &str, _c: &TurnContext) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({"text":text}))
    }
}
#[test]
fn extracts_json_with_runner() {
    zk_kit_extractor::set_runner(Arc::new(R));
    let ctx = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: ".".to_owned(),
    };
    let out = zk_tool_extract::tool()
        .call_json(&ctx, serde_json::json!({"text":"hello"}))
        .unwrap();
    assert!(out.contains("hello"));
}
