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
        schedule: zk_contract_config::ScheduleSettings::default(),
    };
    agent.configure_agent(&settings).unwrap();
    assert_eq!(agent.config().max_steps, 9);

    let reply = agent.run_in(&context(), "hi").unwrap();
    let contract_usage = reply.usage.expect("usage");
    assert_eq!(contract_usage.input_tokens, 7);
    assert_eq!(contract_usage.output_tokens, 3);
    assert_eq!(contract_usage.total_tokens, 10);

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

#[test]
fn usage_aggregates_across_all_model_hops() {
    // rig 0.45's PromptResponse::usage() is the aggregate across the whole run
    // (documented in rig-agent), so a multi-step tool loop must sum both hops.
    let model = MockCompletionModel::from_turns([
        MockTurn::tool_call("call-1", "fs.read", serde_json::json!({"path": "a"}))
            .with_usage(usage(3, 2)),
        MockTurn::text("done").with_usage(usage(5, 4)),
    ]);
    let agent = RigAgent::new(model.into(), vec![Arc::new(ReadTool)])
        .unwrap()
        .with_streaming(false);
    let reply = agent.run_in(&context(), "read").unwrap();
    let usage = reply.usage.expect("usage");
    assert_eq!(usage.input_tokens, 8, "sum over both model hops");
    assert_eq!(usage.output_tokens, 6, "sum over both model hops");
    assert_eq!(usage.total_tokens, 14);
}

struct SlowTool;

impl zk_contract_tool::Tool for SlowTool {
    fn spec(&self) -> zk_contract_tool::ToolSpec {
        zk_contract_tool::ToolSpec {
            name: "slow.echo".to_owned(),
            description: "slow".to_owned(),
            parameters: serde_json::json!({"type":"object"}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, zk_contract_tool::ToolError> {
        Ok("unused".to_owned())
    }
    fn call_json(
        &self,
        _context: &TurnContext,
        _args: serde_json::Value,
    ) -> Result<String, zk_contract_tool::ToolError> {
        std::thread::sleep(std::time::Duration::from_millis(1_300));
        Ok("late".to_owned())
    }
}

#[test]
fn configured_tool_timeout_is_enforced_per_turn() {
    let model = MockCompletionModel::from_turns([
        MockTurn::tool_call("call-slow", "slow.echo", serde_json::json!({})),
        MockTurn::text("done"),
    ]);
    let agent = RigAgent::new(model.into(), vec![Arc::new(SlowTool)])
        .unwrap()
        .with_streaming(false)
        .with_agent_config(zk_agent_rig::AgentRuntimeConfig {
            tools: zk_contract_config::ToolSettings {
                tool_timeout_ms: zk_contract_config::ToolSettings::MIN_TOOL_TIMEOUT_MS,
                approval_timeout_ms: zk_contract_config::ToolSettings::default()
                    .approval_timeout_ms,
                approval_policy: None,
            },
            ..zk_agent_rig::AgentRuntimeConfig::default()
        });
    let reply = agent.run_in(&context(), "use slow").unwrap();
    let result = reply
        .tool_calls
        .first()
        .map(|call| call.result.as_str())
        .unwrap_or_default();
    assert!(
        result.contains("timed out"),
        "configured 1000ms tool timeout must abort the 1300ms tool: {result}"
    );
}

#[test]
fn desktop_ipc_apply_sequence_keeps_env_authoritative() {
    use zk_contract_config::{AgentSettings, AppSettings, RuntimeSettings, ToolSettings};

    // Emulates `apps/desktop/src/ipc.rs` apply path: the legacy registry is only
    // written for a non-default agent section (cleared otherwise), and the
    // runtime snapshot carries only explicitly non-default sections.
    fn desktop_apply_ipc(settings: &AppSettings) {
        if settings.agent == AgentSettings::default() {
            zk_contract_agent::clear_default_agent_settings().unwrap();
        } else {
            zk_contract_agent::set_default_agent_settings(&settings.agent).unwrap();
        }
        zk_contract_config::set_runtime_settings(&RuntimeSettings {
            agent: (settings.agent != AgentSettings::default()).then(|| settings.agent.clone()),
            tools: (settings.tools != ToolSettings::default()).then_some(settings.tools),
            web_search: (settings.web_search != zk_contract_config::WebSearchSettings::default())
                .then(|| settings.web_search.clone()),
        })
        .unwrap();
    }

    zk_contract_config::clear_runtime_settings().unwrap();
    zk_contract_agent::clear_default_agent_settings().unwrap();
    std::env::set_var("ZEOKEEL_AGENT_MAX_STEPS", "21");
    std::env::set_var("ZEOKEEL_AGENT_TEMPERATURE", "0.31");

    // Tools-only save: registry cleared + snapshot agent None -> env wins.
    let mut tools_only = AppSettings::default();
    tools_only.tools.tool_timeout_ms = 2_000;
    desktop_apply_ipc(&tools_only);
    assert!(zk_contract_agent::default_agent_settings().is_none());
    let config = default_agent_config();
    assert_eq!(config.max_steps, 21, "env agent survives a tools-only save");
    assert_eq!(config.completion.temperature, Some(0.31));
    assert_eq!(config.tools.tool_timeout_ms, 2_000);

    // Non-default agent save wins over env...
    let mut custom = tools_only.clone();
    custom.agent.max_steps = 7;
    desktop_apply_ipc(&custom);
    let config = default_agent_config();
    assert_eq!(config.max_steps, 7);
    assert_eq!(config.tools.tool_timeout_ms, 2_000);

    // ...and reset-to-default returns to the env fallback.
    desktop_apply_ipc(&AppSettings::default());
    assert!(zk_contract_agent::default_agent_settings().is_none());
    let config = default_agent_config();
    assert_eq!(config.max_steps, 21);
    assert_eq!(config.completion.temperature, Some(0.31));
    assert_eq!(
        config.tools.tool_timeout_ms,
        ToolSettings::default().tool_timeout_ms,
        "reset-to-default returns to default timeouts"
    );

    zk_contract_config::clear_runtime_settings().unwrap();
    zk_contract_agent::clear_default_agent_settings().unwrap();
    std::env::remove_var("ZEOKEEL_AGENT_MAX_STEPS");
    std::env::remove_var("ZEOKEEL_AGENT_TEMPERATURE");
}

struct CollectSink(std::sync::Mutex<Vec<zk_contract_agent::AgentEvent>>);

impl zk_contract_agent::EventSink for CollectSink {
    fn emit(&self, event: zk_contract_agent::AgentEvent) {
        self.0.lock().expect("sink lock").push(event);
    }
}

#[test]
fn usage_event_is_emitted_once_before_turn_completed() {
    let model = MockCompletionModel::from_turns([MockTurn::text("done").with_usage(usage(7, 3))]);
    let agent = RigAgent::new(model.into(), vec![Arc::new(ReadTool)])
        .unwrap()
        .with_streaming(false);
    let sink = CollectSink(std::sync::Mutex::new(Vec::new()));
    let reply = agent
        .run_stream_in(&context(), "turn-1", "hi", &sink)
        .expect("turn");
    assert_eq!(reply.usage.expect("usage in reply").input_tokens, 7);
    let events = sink.0.lock().expect("events");
    let usage_events: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            zk_contract_agent::AgentEvent::Usage { usage } => Some(*usage),
            _ => None,
        })
        .collect();
    assert_eq!(usage_events.len(), 1, "exactly one Usage event per turn");
    assert_eq!(usage_events[0].input_tokens, 7);
    assert_eq!(usage_events[0].output_tokens, 3);
    assert_eq!(usage_events[0].total_tokens, 10);
    let usage_index = events
        .iter()
        .position(|event| matches!(event, zk_contract_agent::AgentEvent::Usage { .. }))
        .expect("usage position");
    let completed_index = events
        .iter()
        .position(|event| matches!(event, zk_contract_agent::AgentEvent::TurnCompleted { .. }))
        .expect("completed position");
    assert!(
        usage_index < completed_index,
        "Usage precedes TurnCompleted"
    );
}

#[test]
fn replay_history_is_forwarded_to_the_rig_request() {
    use zk_contract_agent::{HistoryMessage, HistoryRole};
    let model = MockCompletionModel::from_turns([MockTurn::text("ok").with_usage(usage(1, 1))]);
    let agent = RigAgent::new(model.clone().into(), vec![Arc::new(ReadTool)])
        .unwrap()
        .with_streaming(false);
    let history = vec![
        HistoryMessage {
            role: HistoryRole::User,
            content: "earlier question".to_owned(),
        },
        HistoryMessage {
            role: HistoryRole::Assistant,
            content: "earlier answer".to_owned(),
        },
    ];
    let sink = CollectSink(std::sync::Mutex::new(Vec::new()));
    agent
        .run_stream_in_shared_with_history(&context(), "turn-h", "next", &history, Arc::new(sink))
        .expect("turn with history");
    let request = model.requests().first().cloned().expect("request");
    let rendered = format!("{:?}", request.chat_history);
    assert!(
        rendered.contains("earlier question"),
        "history user message sent: {rendered}"
    );
    assert!(
        rendered.contains("earlier answer"),
        "history assistant message sent: {rendered}"
    );
    assert!(rendered.contains("next"), "current prompt sent: {rendered}");
}
