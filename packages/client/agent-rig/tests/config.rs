#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only

//! Contract → runtime config → rig request, usage in AgentReply, and the
//! process-wide default re-read on every turn (desktop settings loader path).

use std::sync::Arc;

use rig_core::completion::Usage;
use rig_core::test_utils::{MockCompletionModel, MockTurn};
use zk_agent_rig::{default_agent_config, set_default_agent_config, RigAgent};
use zk_contract_agent::Agent;
use zk_contract_config::{
    AgentSettings, CompletionSettings, HookSettings, MemorySettings, ToolChoiceSetting,
};
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

fn usage(input: u64, output: u64) -> Usage {
    let mut usage = Usage::default();
    usage.input_tokens = Some(input);
    usage.output_tokens = Some(output);
    usage.total_tokens = Some(input.saturating_add(output));
    usage
}

#[test]
fn contract_settings_apply_to_the_next_rig_request() {
    let model = MockCompletionModel::from_turns([MockTurn::text("done").with_usage(usage(7, 3))]);
    let agent = RigAgent::new(model.clone().into(), vec![Arc::new(ReadTool)])
        .unwrap()
        .with_streaming(false);
    let settings = AgentSettings {
        max_steps: 9,
        completion: CompletionSettings {
            temperature: Some(0.4),
            max_tokens: Some(256),
            top_p: Some(0.8),
            seed: Some(11),
            stop: vec!["STOP".to_owned()],
            tool_choice: ToolChoiceSetting::Specific(vec!["fs.read".to_owned()]),
            parallel_tool_calls: Some(true),
            reasoning_effort: Some("medium".to_owned()),
            prompt_cache: Some("short".to_owned()),
        },
        memory: MemorySettings {
            enabled: false,
            dir: String::new(),
        },
        hooks: HookSettings {
            enabled: false,
            log: String::new(),
        },
    };
    agent.configure_agent(&settings).unwrap();
    assert_eq!(agent.config().max_steps, 9);

    let reply = agent.run_in(&context(), "hi").unwrap();
    let contract_usage = reply.usage.expect("usage");
    assert_eq!(contract_usage.input_tokens, Some(7));
    assert_eq!(contract_usage.output_tokens, Some(3));
    assert_eq!(contract_usage.total_tokens, Some(10));

    let request = model.requests().first().cloned().expect("request captured");
    assert_eq!(request.temperature, Some(0.4));
    assert_eq!(request.max_tokens, Some(256));
    assert_eq!(request.options.top_p, Some(0.8));
    assert_eq!(request.options.seed, Some(11));
    assert_eq!(request.options.stop, vec!["STOP".to_owned()]);
    assert_eq!(request.options.parallel_tool_calls, Some(true));
    assert!(request.options.reasoning.is_some());
    assert!(request.options.cache.is_some());
    assert!(matches!(
        request.tool_choice,
        Some(rig_core::message::ToolChoice::Specific { .. })
    ));
}

#[test]
fn global_default_is_reread_every_turn() {
    let previous = default_agent_config();
    let model = MockCompletionModel::from_turns([MockTurn::text("one"), MockTurn::text("two")]);
    let agent = RigAgent::new(model.clone().into(), vec![Arc::new(ReadTool)])
        .unwrap()
        .with_streaming(false);
    let first = AgentSettings {
        completion: CompletionSettings {
            temperature: Some(0.2),
            ..CompletionSettings::default()
        },
        ..AgentSettings::default()
    };
    set_default_agent_config(zk_agent_rig::AgentRuntimeConfig::from_contract(&first).unwrap())
        .unwrap();
    agent.run_in(&context(), "first").unwrap();

    // A later global update (desktop settings.set) must apply to the next turn
    // without rebuilding the agent.
    let second = AgentSettings {
        completion: CompletionSettings {
            temperature: Some(0.6),
            ..CompletionSettings::default()
        },
        ..AgentSettings::default()
    };
    set_default_agent_config(zk_agent_rig::AgentRuntimeConfig::from_contract(&second).unwrap())
        .unwrap();
    agent.run_in(&context(), "second").unwrap();

    let requests = model.requests();
    assert_eq!(requests[0].temperature, Some(0.2));
    assert_eq!(requests[1].temperature, Some(0.6));
    // Restore the process-wide default so sibling tests see the built-in one.
    set_default_agent_config(previous).unwrap();
}
