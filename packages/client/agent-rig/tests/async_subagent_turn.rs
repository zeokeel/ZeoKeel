//! Real agent-turn async subagent integration: a replay model drives a parent
//! turn that starts a background child; the child runs on a real thread through
//! the installed `RigSubagentRunner`, and the control tools (`subagent_status`,
//! `subagent_interrupt`) observe/cancel it. Asserts final state, live-thread
//! count and control-registry consistency.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)] // test-only

use std::sync::Arc;
use std::time::{Duration, Instant};

use rig_core::test_utils::{MockCompletionModel, MockTurn};
use zk_agent_rig::agent;
use zk_contract_agent::Agent;
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

struct SlowTool;

impl Tool for SlowTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "slow.echo".to_owned(),
            description: "test-only slow tool (keeps a child thread alive)".to_owned(),
            parameters: serde_json::json!({"type":"object"}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Ok("slow".to_owned())
    }
    fn call_json(
        &self,
        _context: &TurnContext,
        _args: serde_json::Value,
    ) -> Result<String, ToolError> {
        std::thread::sleep(Duration::from_millis(1_000));
        Ok("slow".to_owned())
    }
}

fn context() -> TurnContext {
    TurnContext {
        conversation_id: "integration-parent".to_owned(),
        workspace_id: "integration-workspace".to_owned(),
        workspace_root: ".".to_owned(),
    }
}

fn tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(zk_tool_subagent::tool()),
        Arc::new(zk_tool_subagent::start_tool()),
        Arc::new(zk_tool_subagent_control::tool()),
        Arc::new(zk_tool_subagent_control::status_tool()),
        Arc::new(zk_tool_subagent_control::interrupt_tool()),
        Arc::new(SlowTool),
    ]
}

fn call_tool(tool: &dyn Tool, args: serde_json::Value) -> serde_json::Value {
    let text = tool.call_json(&context(), args).expect("tool call");
    serde_json::from_str(&text).expect("tool JSON")
}

fn status_of(id: &str) -> Option<serde_json::Value> {
    let value = call_tool(
        &zk_tool_subagent_control::status_tool(),
        serde_json::json!({"id": id}),
    );
    value.get("agent").cloned()
}

fn wait_status(id: &str, wanted: &[&str]) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = status_of(id) {
            let current = status
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if wanted.contains(&current) {
                return status;
            }
        }
        assert!(
            Instant::now() < deadline,
            "child {id} did not reach {wanted:?}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn wait_live_zero() {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let live = zk_kit_subagent::runner().and_then(|runner| runner.live_count());
        if live == Some(0) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "live child threads leaked: {live:?}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn registry_status(id: &str) -> Option<String> {
    zk_kit_subagent::list()
        .into_iter()
        .find(|agent| agent.id == id)
        .map(|agent| agent.status)
}

#[test]
fn parent_turn_drives_async_subagent_start_status_and_interrupt() {
    // Phase A: a replay parent model calls subagent_start; the child executes on
    // a real thread and the control tool observes `done`.
    let tools = tools();
    let parent_model = MockCompletionModel::from_turns([
        MockTurn::tool_call(
            "start-call",
            "subagent_start",
            serde_json::json!({"prompt": "child task"}),
        ),
        MockTurn::text("parent finished"),
        MockTurn::text("child answer"),
        MockTurn::text("spare"),
        MockTurn::text("spare"),
    ]);
    let parent = agent(tools.clone(), parent_model.into())
        .expect("agent")
        .with_streaming(false);
    let reply = parent
        .run_in(&context(), "delegate a task")
        .expect("parent turn");
    let start_call = reply
        .tool_calls
        .iter()
        .find(|call| call.name == "subagent_start")
        .expect("parent model called subagent_start");
    assert!(
        start_call.result.contains("\"running\""),
        "{}",
        start_call.result
    );
    let started: serde_json::Value = serde_json::from_str(&start_call.result).expect("start JSON");
    let child_id = started
        .get("id")
        .and_then(serde_json::Value::as_str)
        .expect("child id");
    assert!(
        reply.text.contains("parent finished") || reply.text.contains("child answer"),
        "parent reply: {}",
        reply.text
    );

    let status = wait_status(child_id, &["done", "failed"]);
    assert_eq!(
        status.get("status").and_then(serde_json::Value::as_str),
        Some("done"),
        "child should complete: {status}"
    );
    assert_eq!(
        registry_status(child_id).as_deref(),
        Some("done"),
        "kit registry must agree with the control tool"
    );
    wait_live_zero();

    // Phase B: start a fresh child on a slow tool, then interrupt it through the
    // control tool; the terminal state must be cancelled and the slot released.
    let slow_model = MockCompletionModel::from_turns([
        MockTurn::tool_call("slow-call", "slow.echo", serde_json::json!({})),
        MockTurn::text("late child answer"),
    ]);
    let _slow_parent = agent(tools, slow_model.into()).expect("slow agent");
    let started = call_tool(
        &zk_tool_subagent::start_tool(),
        serde_json::json!({"prompt": "interrupt me"}),
    );
    let slow_id = started
        .get("id")
        .and_then(serde_json::Value::as_str)
        .expect("slow child id");
    let live = zk_kit_subagent::runner().and_then(|runner| runner.live_count());
    assert!(
        live.is_some_and(|count| count >= 1),
        "a real child thread must be alive after subagent_start: {live:?}"
    );
    let running = wait_status(slow_id, &["running", "done", "failed", "cancelled"]);
    assert_eq!(
        running.get("status").and_then(serde_json::Value::as_str),
        Some("running"),
        "slow child should still be running: {running}"
    );
    let interrupted = call_tool(
        &zk_tool_subagent_control::interrupt_tool(),
        serde_json::json!({"id": slow_id}),
    );
    assert_eq!(
        interrupted
            .get("interrupted")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
    let cancelled = wait_status(slow_id, &["cancelled", "done", "failed"]);
    assert_eq!(
        cancelled.get("status").and_then(serde_json::Value::as_str),
        Some("cancelled"),
        "interrupt must win: {cancelled}"
    );
    wait_live_zero();
    assert_eq!(
        registry_status(slow_id).as_deref(),
        Some("cancelled"),
        "kit registry must record the cancellation"
    );
}
