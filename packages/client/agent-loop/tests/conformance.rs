//! Conformance + behavior tests for `zk-agent-loop` (role `agent.loop`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::arithmetic_side_effects)] // test-only

use std::sync::{Arc, Mutex};

use zk_agent_loop::{agent, AgentLoop};
use zk_contract_agent::{Agent, AgentError};
use zk_contract_llm::{LlmError, LlmProvider};
use zk_contract_tool::{Tool, ToolError, ToolSpec};

/// Provider with a canned reply that records the prompts it receives.
struct CannedLlm {
    reply: Result<String, LlmError>,
    seen: Mutex<Vec<String>>,
}

impl CannedLlm {
    fn new(reply: &str) -> Arc<Self> {
        Self::with_result(Ok(reply.to_owned()))
    }

    fn with_result(reply: Result<String, LlmError>) -> Arc<Self> {
        Arc::new(Self {
            reply,
            seen: Mutex::new(Vec::new()),
        })
    }
}

impl LlmProvider for CannedLlm {
    fn complete(&self, prompt: &str) -> Result<String, LlmError> {
        self.seen.lock().unwrap().push(prompt.to_owned());
        self.reply.clone()
    }
}

/// Tool that returns a fixed result and reports its declared name.
struct FakeTool {
    name: &'static str,
    reply: Result<String, ToolError>,
}

impl Tool for FakeTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name.to_owned(),
            description: "test fixture".to_owned(),
        }
    }

    fn call(&self, _args: &str) -> Result<String, ToolError> {
        self.reply.clone()
    }
}

fn fs_read_tool(reply: Result<String, ToolError>) -> Arc<dyn Tool> {
    Arc::new(FakeTool {
        name: "fs.read",
        reply,
    })
}

#[test]
fn executes_tool_and_records_the_call() {
    let llm = CannedLlm::new("TOOL fs.read demo-data/hello.txt");
    let tool = fs_read_tool(Ok("Hello ZeoKeel!\n这是一个最小 demo 文件。".to_owned()));
    let loop_: Arc<dyn Agent> = Arc::new(agent(vec![tool], llm.clone()));

    let reply = loop_.run("读取 demo-data/hello.txt").unwrap();

    assert_eq!(reply.tool_calls.len(), 1);
    let call = &reply.tool_calls[0];
    assert_eq!(call.name, "fs.read");
    assert_eq!(call.args, "demo-data/hello.txt");
    assert!(call.result.contains("Hello ZeoKeel!"));
    assert!(reply.text.contains("Hello ZeoKeel!"));

    let prompts = llm.seen.lock().unwrap();
    assert_eq!(prompts.len(), 1);
    assert!(prompts[0].contains("USER_INPUT:读取 demo-data/hello.txt"));
    assert!(prompts[0].contains("TOOL fs.read"));
}

#[test]
fn returns_text_without_tool_calls_when_no_directive() {
    let llm = CannedLlm::new("（mock）收到：你好");
    let loop_ = agent(Vec::new(), llm);

    let reply = loop_.run("你好").unwrap();

    assert_eq!(reply.text, "（mock）收到：你好");
    assert!(reply.tool_calls.is_empty());
}

#[test]
fn missing_tool_is_tool_error() {
    let llm = CannedLlm::new("TOOL fs.read a.txt");
    let loop_ = agent(Vec::new(), llm);

    assert!(matches!(loop_.run("读取 a.txt"), Err(AgentError::Tool(_))));
}

#[test]
fn unsupported_tool_is_tool_error() {
    let llm = CannedLlm::new("TOOL shell.run rm -rf /");
    let loop_ = agent(vec![fs_read_tool(Ok("never".to_owned()))], llm);

    assert!(matches!(
        loop_.run("删掉所有东西"),
        Err(AgentError::Tool(_))
    ));
}

#[test]
fn malformed_directive_is_tool_error() {
    let llm = CannedLlm::new("TOOL fs.read");
    let loop_ = agent(vec![fs_read_tool(Ok("never".to_owned()))], llm);

    assert!(matches!(loop_.run("读取"), Err(AgentError::Tool(_))));
}

#[test]
fn tool_failure_is_recorded_not_propagated() {
    let llm = CannedLlm::new("TOOL fs.read ../Cargo.toml");
    let tool = fs_read_tool(Err(ToolError::Denied("不允许 .. 路径穿越".to_owned())));
    let loop_ = agent(vec![tool], llm);

    let reply = loop_.run("读取 ../Cargo.toml").unwrap();

    assert_eq!(reply.tool_calls.len(), 1);
    assert_eq!(reply.tool_calls[0].name, "fs.read");
    assert_eq!(reply.tool_calls[0].args, "../Cargo.toml");
    assert!(reply.tool_calls[0].result.contains("denied"));
    assert!(reply.text.contains("denied"));
}

#[test]
fn provider_failure_is_llm_error() {
    let llm = CannedLlm::with_result(Err(LlmError::Upstream("boom".to_owned())));
    let loop_ = agent(Vec::new(), llm);

    assert!(matches!(loop_.run("你好"), Err(AgentError::Llm(_))));
}

#[test]
fn loop_is_object_safe_as_contract_agent() {
    let llm = CannedLlm::new("plain");
    let loop_: Arc<dyn Agent> = Arc::new(AgentLoop::new(Vec::new(), llm));
    assert_eq!(loop_.run("hi").unwrap().text, "plain");
}

/// Streaming order: started -> deltas* -> completed, UTF-8 safe.
#[test]
fn streaming_emits_ordered_utf8_events() {
    use std::sync::Mutex;
    use zk_contract_agent::{AgentEvent, EventSink};
    use zk_contract_llm::{LlmError, LlmProvider};

    struct Llm;
    impl LlmProvider for Llm {
        fn complete(&self, _prompt: &str) -> Result<String, LlmError> {
            Ok("你好🙂这是一段用于测试分片的中文和 emoji 混合文本，长度足够产生多个 delta。".to_owned())
        }
    }
    struct Sink(Mutex<Vec<AgentEvent>>);
    impl EventSink for Sink {
        fn emit(&self, event: AgentEvent) {
            if let Ok(mut guard) = self.0.lock() { guard.push(event); }
        }
    }
    let loop_ = zk_agent_loop::AgentLoop::new(vec![], std::sync::Arc::new(Llm));
    let sink = Sink(Mutex::new(Vec::new()));
    let reply = loop_.run_stream("t1", "hi", &sink).unwrap();
    let events = sink.0.lock().unwrap();
    assert!(matches!(events.first(), Some(AgentEvent::TurnStarted { .. })));
    assert!(matches!(events.last(), Some(AgentEvent::TurnCompleted { .. })));
    let mut joined = String::new();
    for event in events.iter() {
        if let AgentEvent::TextDelta { delta } = event { joined.push_str(delta); }
    }
    assert_eq!(joined, reply.text);
}
