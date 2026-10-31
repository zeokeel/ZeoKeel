#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use zk_contract_tool::{Tool, TurnContext};
use zk_tool_goal::driver::{GoalSnapshot, GoalTurnRunner, RoundOutcome, RoundUsage};

/// Tests share the process-global goal turn runner; serialize them.
static LOCK: Mutex<()> = Mutex::new(());

fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

struct ScriptedRunner {
    plan: Mutex<VecDeque<Result<RoundOutcome, String>>>,
    rounds: Mutex<Vec<u32>>,
}

impl GoalTurnRunner for ScriptedRunner {
    fn run_round(
        &self,
        _context: &TurnContext,
        _goal: &GoalSnapshot,
        round: u32,
    ) -> Result<RoundOutcome, String> {
        self.rounds.lock().unwrap().push(round);
        self.plan
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Ok(RoundOutcome {
                summary: "(scripted default)".to_owned(),
                achieved: false,
                usage: None,
            }))
    }
}

fn outcome(summary: &str, achieved: bool) -> Result<RoundOutcome, String> {
    Ok(RoundOutcome {
        summary: summary.to_owned(),
        achieved,
        usage: Some(RoundUsage {
            input_tokens: 10,
            output_tokens: 2,
        }),
    })
}

fn install(plan: Vec<Result<RoundOutcome, String>>) -> Arc<ScriptedRunner> {
    let runner = Arc::new(ScriptedRunner {
        plan: Mutex::new(VecDeque::from(plan)),
        rounds: Mutex::new(Vec::new()),
    });
    zk_tool_goal::driver::set_turn_runner(Some(runner.clone()));
    runner
}

fn ctx(tag: &str) -> TurnContext {
    let dir = std::env::temp_dir().join(format!(
        "zk-goal-{tag}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    TurnContext {
        conversation_id: format!("conv-{tag}"),
        workspace_id: "w".to_owned(),
        workspace_root: dir.display().to_string(),
    }
}

fn call(tool: &zk_tool_goal::GoalTool, ctx: &TurnContext, args: serde_json::Value) -> String {
    tool.call_json(ctx, args).unwrap()
}

fn run(tool: &zk_tool_goal::GoalTool, ctx: &TurnContext) -> serde_json::Value {
    serde_json::from_str(&call(tool, ctx, serde_json::json!({"action":"run"}))).unwrap()
}

fn status(tool: &zk_tool_goal::GoalTool, ctx: &TurnContext) -> serde_json::Value {
    serde_json::from_str(&call(tool, ctx, serde_json::json!({"action":"status"}))).unwrap()
}

fn field(value: &serde_json::Value, key: &str) -> serde_json::Value {
    value.get(key).cloned().unwrap()
}

#[test]
fn one_round_per_call_and_max_rounds_stop() {
    let _guard = test_lock();
    let ctx = ctx("max");
    let runner = install(vec![
        outcome("round one", false),
        outcome("round two", false),
    ]);
    let tool = zk_tool_goal::tool();
    call(
        &tool,
        &ctx,
        serde_json::json!({"action":"set","objective":"ship","max_rounds":2}),
    );
    let first = run(&tool, &ctx);
    assert_eq!(field(&first, "executed"), serde_json::json!(true));
    assert_eq!(field(&first, "round"), serde_json::json!(1));
    assert_eq!(field(&first, "status"), serde_json::json!("active"));
    let second = run(&tool, &ctx);
    assert_eq!(field(&second, "round"), serde_json::json!(2));
    assert_eq!(field(&second, "status"), serde_json::json!("blocked"));
    let stopped = run(&tool, &ctx);
    assert_eq!(field(&stopped, "executed"), serde_json::json!(false));
    assert_eq!(field(&stopped, "status"), serde_json::json!("blocked"));
    assert!(field(&stopped, "reason")
        .as_str()
        .unwrap()
        .contains("blocked"));
    assert_eq!(*runner.rounds.lock().unwrap(), vec![1, 2]);
    zk_tool_goal::driver::set_turn_runner(None);
}

#[test]
fn achieved_round_stops_the_driver() {
    let _guard = test_lock();
    let ctx = ctx("achieved");
    let _runner = install(vec![outcome("not yet", false), outcome("done", true)]);
    let tool = zk_tool_goal::tool();
    call(
        &tool,
        &ctx,
        serde_json::json!({"action":"set","objective":"ship","max_rounds":5}),
    );
    assert_eq!(
        field(&run(&tool, &ctx), "status"),
        serde_json::json!("active")
    );
    assert_eq!(
        field(&run(&tool, &ctx), "status"),
        serde_json::json!("achieved")
    );
    let stopped = run(&tool, &ctx);
    assert_eq!(field(&stopped, "executed"), serde_json::json!(false));
    assert!(field(&stopped, "reason")
        .as_str()
        .unwrap()
        .contains("achieved"));
    let goal = status(&tool, &ctx);
    assert_eq!(field(&goal, "status"), serde_json::json!("achieved"));
    assert_eq!(field(&goal, "rounds"), serde_json::json!(2));
    zk_tool_goal::driver::set_turn_runner(None);
}

#[test]
fn cancel_stops_the_driver() {
    let _guard = test_lock();
    let ctx = ctx("cancel");
    let _runner = install(vec![outcome("partial", false)]);
    let tool = zk_tool_goal::tool();
    call(
        &tool,
        &ctx,
        serde_json::json!({"action":"set","objective":"ship"}),
    );
    call(&tool, &ctx, serde_json::json!({"action":"run"}));
    let cancelled = call(&tool, &ctx, serde_json::json!({"action":"cancel"}));
    assert!(cancelled.contains("cancelled"));
    let stopped = run(&tool, &ctx);
    assert_eq!(field(&stopped, "executed"), serde_json::json!(false));
    assert_eq!(field(&stopped, "status"), serde_json::json!("cancelled"));
    zk_tool_goal::driver::set_turn_runner(None);
}

#[test]
fn restart_resumes_the_same_round_once() {
    let _guard = test_lock();
    let ctx = ctx("resume");
    let runner = install(vec![
        Err("simulated crash".to_owned()),
        outcome("resumed", false),
    ]);
    let tool = zk_tool_goal::tool();
    call(
        &tool,
        &ctx,
        serde_json::json!({"action":"set","objective":"ship","max_rounds":3}),
    );
    let error = tool
        .call_json(&ctx, serde_json::json!({"action":"run"}))
        .unwrap_err()
        .to_string();
    assert!(error.contains("goal round 1 failed"), "unexpected: {error}");
    let crashed = status(&tool, &ctx);
    assert_eq!(field(&crashed, "rounds"), serde_json::json!(1));
    assert_eq!(field(&crashed, "in_flight_round"), serde_json::json!(1));
    // Restart/resume: the same round is retried, not skipped or double-counted.
    let resumed = run(&tool, &ctx);
    assert_eq!(field(&resumed, "executed"), serde_json::json!(true));
    assert_eq!(field(&resumed, "round"), serde_json::json!(1));
    assert_eq!(field(&resumed, "rounds"), serde_json::json!(1));
    assert_eq!(*runner.rounds.lock().unwrap(), vec![1, 1]);
    let next = run(&tool, &ctx);
    assert_eq!(field(&next, "round"), serde_json::json!(2));
    zk_tool_goal::driver::set_turn_runner(None);
}

#[test]
fn missing_runtime_is_explicit_and_consumes_no_round() {
    let _guard = test_lock();
    let ctx = ctx("noruntime");
    zk_tool_goal::driver::set_turn_runner(None);
    let tool = zk_tool_goal::tool();
    call(
        &tool,
        &ctx,
        serde_json::json!({"action":"set","objective":"ship","max_rounds":3}),
    );
    let error = tool
        .call_json(&ctx, serde_json::json!({"action":"run"}))
        .unwrap_err()
        .to_string();
    assert!(error.contains("runner unavailable"), "unexpected: {error}");
    let goal = status(&tool, &ctx);
    assert_eq!(field(&goal, "rounds"), serde_json::json!(0));
    assert_eq!(field(&goal, "in_flight_round"), serde_json::json!(null));
}

#[test]
fn legacy_create_update_and_clamp_stay_compatible() {
    let _guard = test_lock();
    let ctx = ctx("legacy");
    let tool = zk_tool_goal::tool();
    let created = call(
        &tool,
        &ctx,
        serde_json::json!({"action":"create","objective":"ship"}),
    );
    assert!(
        created.contains("\"max_rounds\":10"),
        "unexpected: {created}"
    );
    let clamped_high = call(
        &tool,
        &ctx,
        serde_json::json!({"action":"set","objective":"ship","max_rounds":999}),
    );
    assert!(clamped_high.contains("\"max_rounds\":100"));
    let clamped_low = call(
        &tool,
        &ctx,
        serde_json::json!({"action":"set","objective":"ship","max_rounds":0}),
    );
    assert!(clamped_low.contains("\"max_rounds\":1"));
    let updated = call(
        &tool,
        &ctx,
        serde_json::json!({"action":"update","status":"complete","rounds":1}),
    );
    assert!(
        updated.contains("\"status\":\"achieved\""),
        "unexpected: {updated}"
    );
    assert_eq!(
        field(&status(&tool, &ctx), "status"),
        serde_json::json!("achieved")
    );
    // Goals are conversation-scoped: another conversation sees no goal.
    let mut other = ctx.clone();
    other.conversation_id = "conv-other".to_owned();
    assert_eq!(
        call(&tool, &other, serde_json::json!({"action":"get"})),
        "null"
    );
}
