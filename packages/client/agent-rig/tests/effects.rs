#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only

//! B9 subset: JSONL effect log records resolved rig effects and redacts secrets.
use std::sync::Arc;

use rig_core::test_utils::{MockCompletionModel, MockTurn};
use zk_agent_rig::{AgentRuntimeConfig, EffectsConfig, RigAgent};
use zk_contract_agent::Agent;
use zk_contract_tool::TurnContext;

struct ReadTool;

impl zk_contract_tool::Tool for ReadTool {
    fn spec(&self) -> zk_contract_tool::ToolSpec {
        zk_contract_tool::ToolSpec {
            name: "fs.read".to_owned(),
            description: "read".to_owned(),
            parameters: serde_json::json!({"type":"object"}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, zk_contract_tool::ToolError> {
        Ok("content".to_owned())
    }
    fn call_json(
        &self,
        _context: &TurnContext,
        _args: serde_json::Value,
    ) -> Result<String, zk_contract_tool::ToolError> {
        Ok("content".to_owned())
    }
}

fn context() -> TurnContext {
    TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: ".".to_owned(),
    }
}

fn temp_log_path(label: &str) -> std::path::PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    std::env::temp_dir()
        .join(format!("zk-effect-{label}-{stamp}"))
        .join("effects.jsonl")
}

fn agent_with_effects(path: &std::path::Path, enabled: bool) -> RigAgent {
    let model = MockCompletionModel::from_turns([
        MockTurn::tool_call(
            "call-1",
            "fs.read",
            serde_json::json!({"path": "a", "api_key": "sk-proj-abcdefghijklmnop1234"}),
        ),
        MockTurn::text("done"),
    ]);
    RigAgent::new(model.into(), vec![Arc::new(ReadTool)])
        .unwrap()
        .with_streaming(false)
        .with_agent_config(AgentRuntimeConfig {
            effects: EffectsConfig {
                enabled,
                path: path.to_string_lossy().to_string(),
            },
            ..AgentRuntimeConfig::default()
        })
}

#[test]
fn effect_log_writes_redacted_jsonl_records() {
    let path = temp_log_path("enabled");
    let agent = agent_with_effects(&path, true);
    agent.run_in(&context(), "go").expect("turn");
    let text = std::fs::read_to_string(&path).expect("effect log exists");
    assert!(!text.is_empty(), "effect log has records");
    for line in text.lines() {
        let value: serde_json::Value = serde_json::from_str(line).expect("valid JSON line");
        assert!(value.get("kind").is_some(), "record has a kind: {line}");
    }
    assert!(text.contains("fs.read"), "tool effect recorded: {text}");
    assert!(
        !text.contains("sk-proj-abcdefghijklmnop1234"),
        "secret redacted from effect log: {text}"
    );
    if let Some(parent) = path.parent() {
        let _ = std::fs::remove_dir_all(parent);
    }
}

#[test]
fn disabled_effect_log_writes_nothing() {
    let path = temp_log_path("disabled");
    let agent = agent_with_effects(&path, false);
    agent.run_in(&context(), "go").expect("turn");
    assert!(!path.exists(), "disabled effect log must not create a file");
}
