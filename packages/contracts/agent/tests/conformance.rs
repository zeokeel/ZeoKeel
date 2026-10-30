//! Contract conformance: reply DTO round-trips and traits are object-safe.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

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
            input_tokens: Some(11),
            output_tokens: Some(5),
            total_tokens: Some(16),
            cached_tokens: Some(4),
            reasoning_tokens: Some(2),
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
