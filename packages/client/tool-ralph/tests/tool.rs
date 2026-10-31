#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::sync::{Arc, Mutex};
use zk_contract_tool::{Tool, TurnContext};

/// `zk_kit_subagent::set_runner` is process-global, so tests in this binary must
/// not install runners concurrently (rust test harness runs them in parallel).
static RUNNER_LOCK: Mutex<()> = Mutex::new(());

struct R;
impl zk_kit_subagent::SubagentRunner for R {
    fn run(&self, prompt: &str, _c: &TurnContext, _d: usize) -> Result<String, String> {
        Ok(prompt.to_owned())
    }
}
#[test]
fn ralph_bounded() {
    let _guard = RUNNER_LOCK.lock().expect("runner lock poisoned");
    zk_kit_subagent::set_runner(Arc::new(R));
    let ctx = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: ".".to_owned(),
    };
    let out = zk_tool_ralph::tool()
        .call_json(&ctx, serde_json::json!({"objective":"x","max_rounds":2}))
        .unwrap();
    assert!(out.contains("rounds") && out.contains("round 2"));
}

struct FreshRunner(std::sync::Arc<std::sync::atomic::AtomicUsize>);

impl zk_kit_subagent::SubagentRunner for FreshRunner {
    fn run(&self, _prompt: &str, _c: &TurnContext, _d: usize) -> Result<String, String> {
        Err("ralph must use run_fresh (B4), not run".to_owned())
    }
    fn run_fresh(&self, prompt: &str, _c: &TurnContext, _d: usize) -> Result<String, String> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(format!("fresh:{prompt}"))
    }
}

#[test]
fn ralph_uses_fresh_conversations_per_round() {
    let _guard = RUNNER_LOCK.lock().expect("runner lock poisoned");
    use std::sync::atomic::{AtomicUsize, Ordering};
    let calls = std::sync::Arc::new(AtomicUsize::new(0));
    zk_kit_subagent::set_runner(std::sync::Arc::new(FreshRunner(std::sync::Arc::clone(
        &calls,
    ))));
    let ctx = TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: ".".to_owned(),
    };
    let out = zk_tool_ralph::tool()
        .call_json(&ctx, serde_json::json!({"objective":"x","max_rounds":3}))
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert!(
        out.contains("fresh:"),
        "each round ran with fresh context: {out}"
    );
}
