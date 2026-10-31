//! Bounded goal round driver (spec R9.1/F3).
//!
//! A goal is a conversation-level objective plus a persisted round counter. [`run_round`]
//! executes exactly one agent turn per call; it stops when the objective is achieved, the goal is
//! cancelled, or `max_rounds` (default 10, clamp 1..=100) is reached. The consumed round is
//! persisted *before* the turn runs and cleared after it, so a crash/restart resumes the same
//! round instead of skipping or double-counting it.
//!
//! The agent turn itself is pluggable: a host may install a [`GoalTurnRunner`] with
//! [`set_turn_runner`]; otherwise the shared `zk-kit-subagent` runner (installed by the desktop
//! agent runtime) executes one child agent turn. This module never starts an agent loop itself.
#![forbid(unsafe_code)]
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::{json, Value};
use zk_contract_tool::{ToolError, TurnContext};

/// Default maximum rounds when the caller does not choose one.
pub const DEFAULT_MAX_ROUNDS: u32 = 10;
/// Maximum accepted rounds (`max_rounds` is clamped to `1..=MAX_ROUNDS_LIMIT`).
pub const MAX_ROUNDS_LIMIT: u32 = 100;
/// Bounded per-goal round log (oldest entries are dropped).
pub const ROUND_LOG_LIMIT: usize = 100;
/// Marker a child turn prints when the objective is fully achieved.
pub const ACHIEVED_MARKER: &str = "GOAL_ACHIEVED";
/// Goal store path (workspace-relative), keyed per conversation.
const GOAL_PATH: &str = ".zeokeel/goal.json";
/// Maximum objective length in bytes.
const MAX_OBJECTIVE_BYTES: usize = 4096;

/// One round's token usage, when the turn runner reports it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoundUsage {
    /// Input tokens.
    pub input_tokens: u64,
    /// Output tokens.
    pub output_tokens: u64,
}

/// Result of one goal round.
#[derive(Clone, Debug)]
pub struct RoundOutcome {
    /// Concrete progress/blocker report from the turn.
    pub summary: String,
    /// Whether the objective is now achieved.
    pub achieved: bool,
    /// Token usage for the round, when known.
    pub usage: Option<RoundUsage>,
}

/// Goal state handed to a turn runner (a snapshot, not a live handle).
#[derive(Clone, Debug)]
pub struct GoalSnapshot {
    /// Conversation-level objective.
    pub objective: String,
    /// Current status (`active`/`achieved`/`cancelled`/`blocked`).
    pub status: String,
    /// Round currently being executed.
    pub rounds: u32,
    /// Maximum rounds allowed for this goal.
    pub max_rounds: u32,
    /// Result of the previous round, if any.
    pub last_result: Option<String>,
}

/// Executes exactly one agent turn for a goal round.
///
/// The desktop agent runtime (or a test) installs an implementation with [`set_turn_runner`];
/// implementations must not loop — one call = one turn.
pub trait GoalTurnRunner: Send + Sync {
    /// Run one turn toward `goal` for `round`.
    fn run_round(
        &self,
        context: &TurnContext,
        goal: &GoalSnapshot,
        round: u32,
    ) -> Result<RoundOutcome, String>;
}

static RUNNER: OnceLock<Mutex<Option<Arc<dyn GoalTurnRunner>>>> = OnceLock::new();

fn runner_slot() -> &'static Mutex<Option<Arc<dyn GoalTurnRunner>>> {
    RUNNER.get_or_init(|| Mutex::new(None))
}

/// Install (or clear, with `None`) the goal turn runner.
///
/// Returns `true` when a runner was previously installed.
pub fn set_turn_runner(runner: Option<Arc<dyn GoalTurnRunner>>) -> bool {
    let slot = runner_slot();
    match slot.lock() {
        Ok(mut guard) => {
            let previous = guard.is_some();
            *guard = runner;
            previous
        }
        Err(_) => false,
    }
}

/// Current goal turn runner, if one was installed.
#[must_use]
pub fn turn_runner() -> Option<Arc<dyn GoalTurnRunner>> {
    runner_slot().lock().ok().and_then(|guard| guard.clone())
}

/// Outcome of one [`run_round`] call.
#[derive(Clone, Debug)]
pub struct RunReport {
    /// Whether a round was executed by this call.
    pub executed: bool,
    /// Why no round ran (`status`/`max_rounds`), when `executed == false`.
    pub reason: Option<String>,
    /// Round executed by this call.
    pub round: Option<u32>,
    /// Goal status after the call.
    pub status: String,
    /// Round counter after the call.
    pub rounds: u32,
    /// Maximum rounds for this goal.
    pub max_rounds: u32,
    /// Round result summary after the call.
    pub result: Option<String>,
}

impl RunReport {
    /// JSON view returned by the tool.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "executed": self.executed,
            "reason": self.reason,
            "round": self.round,
            "status": self.status,
            "rounds": self.rounds,
            "max_rounds": self.max_rounds,
            "result": self.result,
        })
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn store_path(context: &TurnContext) -> Result<PathBuf, ToolError> {
    zk_kit_tools::safe_path(context, GOAL_PATH)
}

fn u32_field(goal: &Value, key: &str) -> u32 {
    goal.get(key)
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0)
}

fn string_field<'a>(goal: &'a Value, key: &str) -> Option<&'a str> {
    goal.get(key).and_then(Value::as_str)
}

fn set_field(goal: &mut Value, key: &str, value: Value) {
    if let Some(object) = goal.as_object_mut() {
        object.insert(key.to_owned(), value);
    }
}

fn normalize_status(raw: &str) -> Option<&'static str> {
    match raw {
        "active" => Some("active"),
        "achieved" | "complete" => Some("achieved"),
        "cancelled" | "canceled" => Some("cancelled"),
        "blocked" => Some("blocked"),
        _ => None,
    }
}

fn status_of(goal: &Value) -> String {
    string_field(goal, "status")
        .map(|raw| normalize_status(raw).unwrap_or("active"))
        .unwrap_or("active")
        .to_owned()
}

fn migrate_legacy(raw: &Value) -> Value {
    let status = string_field(raw, "status")
        .map(|raw| normalize_status(raw).unwrap_or("active"))
        .unwrap_or("active");
    let objective = string_field(raw, "objective")
        .unwrap_or_default()
        .to_owned();
    let rounds = u32_field(raw, "rounds");
    json!({
        "objective": objective,
        "status": status,
        "rounds": rounds,
        "max_rounds": DEFAULT_MAX_ROUNDS,
        "in_flight_round": null,
        "last_result": null,
        "last_error": null,
        "last_usage": null,
        "reason": string_field(raw, "reason"),
        "created_ms": raw.get("created_ms").cloned().unwrap_or(Value::Null),
        "updated_ms": now_ms(),
        "round_log": [],
    })
}

fn load_root(context: &TurnContext) -> Result<Value, ToolError> {
    let path = store_path(context)?;
    if !path.is_file() {
        return Ok(json!({"schema": 1, "goals": {}}));
    }
    let raw = zk_kit_tools::read_text(&path)?;
    let parsed: Value = serde_json::from_str(&raw)
        .map_err(|error| ToolError::InvalidArgs(format!("goal store corrupted: {error}")))?;
    if parsed.get("goals").is_some() {
        return Ok(parsed);
    }
    let mut goals = serde_json::Map::new();
    goals.insert(context.conversation_id.clone(), migrate_legacy(&parsed));
    Ok(json!({"schema": 1, "goals": goals}))
}

fn save_root(context: &TurnContext, root: &Value) -> Result<(), ToolError> {
    zk_kit_tools::write_text(&store_path(context)?, &root.to_string())
}

/// Load this conversation's goal, migrating a legacy single-goal file on read.
///
/// # Errors
///
/// Reports workspace/path and JSON corruption errors.
pub fn goal(context: &TurnContext) -> Result<Option<Value>, ToolError> {
    let root = load_root(context)?;
    Ok(root
        .get("goals")
        .and_then(Value::as_object)
        .and_then(|goals| goals.get(&context.conversation_id))
        .cloned())
}

fn save_goal(context: &TurnContext, goal: &Value) -> Result<(), ToolError> {
    let mut root = load_root(context)?;
    let goals = root
        .as_object_mut()
        .and_then(|object| object.get_mut("goals"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| ToolError::InvalidArgs("goal store corrupted".to_owned()))?;
    goals.insert(context.conversation_id.clone(), goal.clone());
    set_field(&mut root, "schema", json!(1));
    save_root(context, &root)
}

/// Create/replace the conversation goal and reset its round counter.
///
/// # Errors
///
/// Reports empty/oversized objectives and persistence errors.
pub fn set_goal(
    context: &TurnContext,
    objective: &str,
    max_rounds: Option<u32>,
) -> Result<Value, ToolError> {
    let objective = objective.trim();
    if objective.is_empty() {
        return Err(ToolError::InvalidArgs("objective is empty".to_owned()));
    }
    if objective.len() > MAX_OBJECTIVE_BYTES {
        return Err(ToolError::InvalidArgs(format!(
            "objective is longer than {MAX_OBJECTIVE_BYTES} bytes"
        )));
    }
    let max_rounds = max_rounds
        .unwrap_or(DEFAULT_MAX_ROUNDS)
        .clamp(1, MAX_ROUNDS_LIMIT);
    let goal = json!({
        "objective": objective,
        "status": "active",
        "rounds": 0,
        "max_rounds": max_rounds,
        "in_flight_round": null,
        "last_result": null,
        "last_error": null,
        "last_usage": null,
        "reason": null,
        "created_ms": now_ms(),
        "updated_ms": now_ms(),
        "round_log": [],
    });
    save_goal(context, &goal)?;
    Ok(goal)
}

/// Update the legacy fields (`status`/`reason`/`rounds`) of the conversation goal.
///
/// # Errors
///
/// Reports a missing goal, invalid status or persistence errors.
pub fn update_goal(
    context: &TurnContext,
    status: Option<&str>,
    reason: Option<&str>,
    rounds: Option<u32>,
) -> Result<Value, ToolError> {
    let mut goal =
        goal(context)?.ok_or_else(|| ToolError::InvalidArgs("no goal; set first".to_owned()))?;
    if let Some(status) = status {
        let normalized = normalize_status(status)
            .ok_or_else(|| ToolError::InvalidArgs(format!("invalid goal status {status}")))?;
        set_field(&mut goal, "status", json!(normalized));
    }
    if let Some(reason) = reason {
        set_field(&mut goal, "reason", json!(reason));
    }
    if let Some(rounds) = rounds {
        let max_rounds = u32_field(&goal, "max_rounds").clamp(1, MAX_ROUNDS_LIMIT);
        set_field(&mut goal, "rounds", json!(rounds.min(max_rounds)));
        set_field(&mut goal, "in_flight_round", Value::Null);
    }
    set_field(&mut goal, "updated_ms", json!(now_ms()));
    save_goal(context, &goal)?;
    Ok(goal)
}

/// Cancel the conversation goal (idempotent; keeps the round counter/log for inspection).
///
/// # Errors
///
/// Reports a missing goal or persistence errors.
pub fn cancel_goal(context: &TurnContext) -> Result<Value, ToolError> {
    let mut goal =
        goal(context)?.ok_or_else(|| ToolError::InvalidArgs("no goal; set first".to_owned()))?;
    set_field(&mut goal, "status", json!("cancelled"));
    set_field(&mut goal, "reason", json!("cancelled"));
    set_field(&mut goal, "in_flight_round", Value::Null);
    set_field(&mut goal, "updated_ms", json!(now_ms()));
    save_goal(context, &goal)?;
    Ok(goal)
}

fn usage_json(usage: Option<RoundUsage>) -> Value {
    match usage {
        Some(usage) => json!({
            "input_tokens": usage.input_tokens,
            "output_tokens": usage.output_tokens,
        }),
        None => Value::Null,
    }
}

fn append_round_log(
    goal: &mut Value,
    round: u32,
    summary: &str,
    usage: Option<RoundUsage>,
    achieved: bool,
) {
    let mut log = goal
        .get("round_log")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    log.push(json!({
        "round": round,
        "at_ms": now_ms(),
        "result": summary,
        "achieved": achieved,
        "usage": usage_json(usage),
    }));
    let excess = log.len().saturating_sub(ROUND_LOG_LIMIT);
    if excess > 0 {
        log.drain(..excess);
    }
    set_field(goal, "round_log", Value::Array(log));
}

fn runner_available() -> bool {
    if turn_runner().is_some() {
        return true;
    }
    zk_kit_subagent::runner().is_some()
        && zk_kit_subagent::current_depth() < zk_kit_subagent::MAX_DEPTH
}

fn round_prompt(goal: &GoalSnapshot, round: u32) -> String {
    let previous = goal.last_result.as_deref().unwrap_or("(no previous round)");
    format!(
        "Goal round {round}/{} for this conversation.\nObjective:\n{}\nPrevious round result:\n{previous}\n\n\
         Work exactly one round toward the objective using the available tools; report concrete \
         progress and blockers. When the objective is fully achieved, include the exact line \
         {ACHIEVED_MARKER} in your final answer.",
        goal.max_rounds, goal.objective
    )
}

fn execute_round(
    context: &TurnContext,
    goal: &GoalSnapshot,
    round: u32,
) -> Result<RoundOutcome, String> {
    if let Some(runner) = turn_runner() {
        return runner.run_round(context, goal, round);
    }
    let runner = zk_kit_subagent::runner().ok_or_else(|| {
        "no goal turn runner installed and no subagent runner is available".to_owned()
    })?;
    let depth = zk_kit_subagent::current_depth();
    if depth >= zk_kit_subagent::MAX_DEPTH {
        return Err(format!(
            "goal round depth limit reached ({depth}/{})",
            zk_kit_subagent::MAX_DEPTH
        ));
    }
    let prompt = round_prompt(goal, round);
    let text = zk_kit_subagent::with_depth(depth.saturating_add(1), || {
        runner.run(&prompt, context, depth)
    })?;
    let achieved = text.lines().any(|line| line.trim() == ACHIEVED_MARKER);
    Ok(RoundOutcome {
        summary: text,
        achieved,
        usage: None,
    })
}

/// Execute exactly one goal round and persist the outcome.
///
/// # Errors
///
/// Explicit `Denied` when no turn runner is installed (state is left unchanged), `InvalidArgs`
/// when there is no goal, and `Denied` when the turn fails (the consumed round stays in flight so
/// the next call resumes it).
pub fn run_round(context: &TurnContext) -> Result<RunReport, ToolError> {
    if !runner_available() {
        return Err(ToolError::Denied(
            "goal round runner unavailable: install a GoalTurnRunner or the agent runtime's subagent runner"
                .to_owned(),
        ));
    }
    let _guard = RUN_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut goal =
        goal(context)?.ok_or_else(|| ToolError::InvalidArgs("no goal; set first".to_owned()))?;
    let status = status_of(&goal);
    let max_rounds = u32_field(&goal, "max_rounds").clamp(1, MAX_ROUNDS_LIMIT);
    let rounds = u32_field(&goal, "rounds").min(max_rounds);
    if status != "active" {
        return Ok(RunReport {
            executed: false,
            reason: Some(format!("goal status is {status}")),
            round: None,
            status,
            rounds,
            max_rounds,
            result: string_field(&goal, "last_result").map(str::to_owned),
        });
    }
    let in_flight = goal
        .get("in_flight_round")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    let round = match in_flight {
        Some(round) => round.min(max_rounds),
        None => {
            if rounds >= max_rounds {
                let final_status = "blocked";
                set_field(&mut goal, "status", json!(final_status));
                set_field(&mut goal, "reason", json!("max_rounds reached"));
                set_field(&mut goal, "in_flight_round", Value::Null);
                set_field(&mut goal, "updated_ms", json!(now_ms()));
                save_goal(context, &goal)?;
                return Ok(RunReport {
                    executed: false,
                    reason: Some(format!("max_rounds reached ({max_rounds})")),
                    round: None,
                    status: final_status.to_owned(),
                    rounds,
                    max_rounds,
                    result: string_field(&goal, "last_result").map(str::to_owned),
                });
            }
            rounds.saturating_add(1)
        }
    };
    // Persist the consumed round before running: crash/restart resumes this round.
    set_field(&mut goal, "rounds", json!(round));
    set_field(&mut goal, "in_flight_round", json!(round));
    set_field(&mut goal, "last_error", Value::Null);
    set_field(&mut goal, "updated_ms", json!(now_ms()));
    save_goal(context, &goal)?;
    let snapshot = GoalSnapshot {
        objective: string_field(&goal, "objective")
            .unwrap_or_default()
            .to_owned(),
        status: status_of(&goal),
        rounds: round,
        max_rounds,
        last_result: string_field(&goal, "last_result").map(str::to_owned),
    };
    let outcome = match execute_round(context, &snapshot, round) {
        Ok(outcome) => outcome,
        Err(message) => {
            // Keep in_flight_round set so the next call retries the same round.
            set_field(&mut goal, "last_error", json!(message));
            set_field(&mut goal, "updated_ms", json!(now_ms()));
            save_goal(context, &goal)?;
            return Err(ToolError::Denied(format!(
                "goal round {round} failed: {message}"
            )));
        }
    };
    let summary = outcome.summary;
    let final_status = if outcome.achieved {
        "achieved"
    } else if round >= max_rounds {
        "blocked"
    } else {
        "active"
    };
    set_field(&mut goal, "in_flight_round", Value::Null);
    set_field(&mut goal, "last_result", json!(summary.clone()));
    set_field(&mut goal, "last_error", Value::Null);
    set_field(&mut goal, "last_usage", usage_json(outcome.usage));
    set_field(&mut goal, "status", json!(final_status));
    set_field(
        &mut goal,
        "reason",
        if outcome.achieved {
            json!("achieved")
        } else if final_status == "blocked" {
            json!("max_rounds reached")
        } else {
            Value::Null
        },
    );
    set_field(&mut goal, "updated_ms", json!(now_ms()));
    append_round_log(&mut goal, round, &summary, outcome.usage, outcome.achieved);
    save_goal(context, &goal)?;
    Ok(RunReport {
        executed: true,
        reason: None,
        round: Some(round),
        status: final_status.to_owned(),
        rounds: round,
        max_rounds,
        result: Some(summary),
    })
}

static RUN_LOCK: Mutex<()> = Mutex::new(());
