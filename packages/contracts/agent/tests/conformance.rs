//! Contract conformance: reply DTO round-trips and traits are object-safe.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use std::sync::Arc;
use zk_contract_agent::{Agent, AgentError, AgentReply, ToolCallRecord};

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
