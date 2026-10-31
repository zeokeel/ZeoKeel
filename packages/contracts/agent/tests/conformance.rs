//! Contract conformance: reply DTO round-trips and traits are object-safe.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only

use std::sync::Arc;
use zk_contract_agent::{Agent, AgentError, AgentReply, AgentUsage, ToolCallRecord};

struct EchoAgent;

impl Agent for EchoAgent {
    fn run(&self, input: &str) -> Result<AgentReply, AgentError> {
        if input.is_empty() {
            return Err(AgentError::Invalid("empty".to_owned()));
        }
        Ok(AgentReply {
            text: input.to_owned(),
            tool_calls: vec![ToolCallRecord {
                name: "demo.echo".to_owned(),
                args: input.to_owned(),
                result: input.to_owned(),
            }],
            usage: None,
        })
    }
}

#[test]
fn agent_reply_round_trips() {
    let agent: Arc<dyn Agent> = Arc::new(EchoAgent);
    let reply = agent.run("hi").unwrap();
    let json = serde_json::to_string(&reply).unwrap();
    let back: AgentReply = serde_json::from_str(&json).unwrap();
    assert_eq!(reply, back);
    assert!(agent.run("").is_err());
}

#[test]
fn agent_usage_round_trips_and_defaults_absent() {
    let reply = AgentReply {
        text: "hi".to_owned(),
        tool_calls: Vec::new(),
        usage: Some(AgentUsage {
            input_tokens: 11,
            output_tokens: 5,
            total_tokens: 16,
            cached_tokens: 4,
            reasoning_tokens: 2,
            cost_micros: Some(42),
        }),
    };
    let json = serde_json::to_string(&reply).unwrap();
    let back: AgentReply = serde_json::from_str(&json).unwrap();
    assert_eq!(reply, back);
    // Backward compatible: a reply without `usage` parses with None.
    let legacy = r#"{"text":"hi","tool_calls":[]}"#;
    let parsed: AgentReply = serde_json::from_str(legacy).unwrap();
    assert_eq!(parsed.usage, None);
}

#[test]
fn configure_agent_default_is_noop() {
    let agent: Arc<dyn Agent> = Arc::new(EchoAgent);
    let settings = zk_contract_config::AgentSettings::default();
    agent.configure_agent(&settings).unwrap();
}

#[test]
fn agent_registry_can_be_cleared_for_reset_to_default() {
    let settings = zk_contract_config::AgentSettings {
        max_steps: 12,
        ..zk_contract_config::AgentSettings::default()
    };
    zk_contract_agent::set_default_agent_settings(&settings).unwrap();
    assert_eq!(
        zk_contract_agent::default_agent_settings()
            .unwrap()
            .max_steps,
        12
    );
    zk_contract_agent::clear_default_agent_settings().unwrap();
    assert!(zk_contract_agent::default_agent_settings().is_none());
}

#[test]
fn usage_event_round_trips_additively() {
    use zk_contract_agent::{AgentEvent, AgentUsage};
    let usage = AgentUsage {
        input_tokens: 12,
        output_tokens: 7,
        total_tokens: 19,
        cached_tokens: 2,
        reasoning_tokens: 1,
        cost_micros: None,
    };
    let event = AgentEvent::Usage { usage };
    let wire = serde_json::to_value(&event).expect("serialize usage event");
    assert_eq!(wire["type"], "usage");
    assert_eq!(wire["usage"]["input_tokens"], 12);
    assert_eq!(wire["usage"]["total_tokens"], 19);
    let decoded: AgentEvent = serde_json::from_value(wire).expect("deserialize usage event");
    assert_eq!(decoded, event);
    // Additive: existing consumers that deserialize older variants keep working.
    let legacy: AgentEvent = serde_json::from_value(serde_json::json!({"type": "turn_cancelled"}))
        .expect("legacy event");
    assert_eq!(legacy, AgentEvent::TurnCancelled);
}
