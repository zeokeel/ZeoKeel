//! Coordinated agent team: the parent agent acts as coordinator, dispatches a
//! bounded set of workers over the shared `zk-kit-subagent` async runner, and
//! aggregates their per-worker status/results (ADR-070).
#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

/// Hard per-call worker cap (same bound as the runner's async concurrency cap).
const MAX_WORKERS: usize = zk_kit_subagent::MAX_ASYNC_AGENTS;
/// Default bounded wait before still-running workers are interrupted.
const DEFAULT_TIMEOUT_MS: u64 = 60_000;
/// Lower bound for `timeout_ms`.
const MIN_TIMEOUT_MS: u64 = 100;
/// Upper bound for `timeout_ms`; keep it below the agent tool timeout budget.
const MAX_TIMEOUT_MS: u64 = 300_000;
/// Poll interval for worker status/result.
const POLL_INTERVAL_MS: u64 = 50;

type Runner = Arc<dyn zk_kit_subagent::SubagentRunner>;

/// Agent team tool (role `tool.agent_team`).
pub struct AgentTeamTool;

#[must_use]
pub fn tool() -> AgentTeamTool {
    AgentTeamTool
}

struct Task {
    id: String,
    prompt: String,
}

struct Plan {
    objective: String,
    tasks: Vec<Task>,
    max_workers: usize,
    timeout_ms: u64,
}

struct StartedWorker {
    id: String,
    task_id: String,
}

struct WorkerReport {
    worker: String,
    task: String,
    status: String,
    result: Option<String>,
    error: Option<String>,
}

fn invalid(message: &str) -> ToolError {
    ToolError::InvalidArgs(message.to_owned())
}

fn non_empty_str<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

fn parse_plan(args: &serde_json::Value) -> Result<Plan, ToolError> {
    let objective = non_empty_str(args, "objective")
        .ok_or_else(|| invalid("agent_team requires a non-empty objective"))?
        .to_owned();
    let items = args
        .get("tasks")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| invalid("agent_team requires a tasks array"))?;
    if items.is_empty() {
        return Err(invalid("agent_team tasks must not be empty"));
    }
    if items.len() > MAX_WORKERS {
        return Err(ToolError::InvalidArgs(format!(
            "agent_team supports at most {MAX_WORKERS} tasks per call ({})",
            items.len()
        )));
    }
    let mut tasks = Vec::with_capacity(items.len());
    let mut ids = BTreeSet::new();
    for (index, item) in items.iter().enumerate() {
        let prompt = non_empty_str(item, "prompt")
            .ok_or_else(|| invalid("each agent_team task requires a non-empty prompt"))?
            .to_owned();
        let id = non_empty_str(item, "id").unwrap_or("").to_owned();
        let id = if id.is_empty() {
            format!("worker-{}", index.saturating_add(1))
        } else {
            id
        };
        if !ids.insert(id.clone()) {
            return Err(ToolError::InvalidArgs(format!(
                "agent_team has a duplicate task id: {id}"
            )));
        }
        tasks.push(Task { id, prompt });
    }
    let max_workers = match args.get("max_workers") {
        None => tasks.len(),
        Some(value) => {
            let raw = value
                .as_u64()
                .ok_or_else(|| invalid("max_workers must be an integer"))?;
            usize::try_from(raw).map_err(|_| invalid("max_workers is out of range"))?
        }
    };
    if !(1..=MAX_WORKERS).contains(&max_workers) {
        return Err(ToolError::InvalidArgs(format!(
            "max_workers must be between 1 and {MAX_WORKERS}"
        )));
    }
    if tasks.len() > max_workers {
        return Err(ToolError::InvalidArgs(format!(
            "agent_team has {} tasks but max_workers is {max_workers}",
            tasks.len()
        )));
    }
    let timeout_ms = match args.get("timeout_ms") {
        None => DEFAULT_TIMEOUT_MS,
        Some(value) => value
            .as_u64()
            .ok_or_else(|| invalid("timeout_ms must be an integer"))?
            .clamp(MIN_TIMEOUT_MS, MAX_TIMEOUT_MS),
    };
    Ok(Plan {
        objective,
        tasks,
        max_workers,
        timeout_ms,
    })
}

fn worker_prompt(objective: &str, task: &Task) -> String {
    format!(
        "You are worker {} on a coordinated agent team. Shared objective:\n{objective}\n\nYour assigned task:\n{}\n\nWork independently with your tools toward the task. Report the concrete result and any blockers.",
        task.id, task.prompt
    )
}

fn report_for(runner: &Runner, worker: &StartedWorker) -> WorkerReport {
    let Some(status) = runner.status(&worker.id) else {
        return WorkerReport {
            worker: worker.id.clone(),
            task: worker.task_id.clone(),
            status: "unknown".to_owned(),
            result: None,
            error: Some("worker record is no longer available".to_owned()),
        };
    };
    let outcome = runner.result(&worker.id);
    let mut report = WorkerReport {
        worker: worker.id.clone(),
        task: worker.task_id.clone(),
        status: status.status,
        result: None,
        error: None,
    };
    match outcome {
        Some(Ok(text)) => report.result = Some(text),
        Some(Err(error)) => report.error = Some(error),
        None => {
            if report.status != "running" {
                report.error = Some(format!(
                    "worker finished without a result ({})",
                    report.status
                ));
            }
        }
    }
    report
}

fn interrupt_all(runner: &Runner, workers: &[StartedWorker]) {
    for worker in workers {
        if let Err(error) = runner.interrupt(&worker.id) {
            // A worker that finished between reporting and interrupt is not an error.
            if !error.contains("already finished") {
                eprintln!(
                    "zk-tool-agent-team: interrupt worker {} failed: {error}",
                    worker.id
                );
            }
        }
    }
}

fn worker_json(report: &WorkerReport) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    object.insert(
        "id".to_owned(),
        serde_json::Value::String(report.worker.clone()),
    );
    object.insert(
        "task".to_owned(),
        serde_json::Value::String(report.task.clone()),
    );
    object.insert(
        "status".to_owned(),
        serde_json::Value::String(report.status.clone()),
    );
    if let Some(result) = &report.result {
        object.insert(
            "result".to_owned(),
            serde_json::Value::String(result.clone()),
        );
    }
    if let Some(error) = &report.error {
        object.insert("error".to_owned(), serde_json::Value::String(error.clone()));
    }
    serde_json::Value::Object(object)
}

impl Tool for AgentTeamTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "agent_team".to_owned(),
            description: "Coordinate a bounded team of worker subagents over one objective: dispatch up to 8 workers in parallel, wait for their per-worker results, and aggregate status/errors back to the parent (no dynamic team UI).".to_owned(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "objective": {
                        "type": "string",
                        "description": "Shared objective given to every worker."
                    },
                    "tasks": {
                        "type": "array",
                        "description": "Per-worker tasks (max 8).",
                        "minItems": 1,
                        "maxItems": 8,
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": {"type": "string", "description": "Optional stable task id (default worker-N)."},
                                "prompt": {"type": "string", "description": "What this worker should do."}
                            },
                            "required": ["prompt"],
                            "additionalProperties": false
                        }
                    },
                    "max_workers": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 8,
                        "description": "Maximum workers dispatched (default: number of tasks)."
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "minimum": 100,
                        "maximum": 300000,
                        "description": "Bounded wait before running workers are interrupted (default 60000; must stay below the agent tool timeout)."
                    }
                },
                "required": ["objective", "tasks"],
                "additionalProperties": false
            }),
            mutating: false,
            requires_approval: false,
        }
    }

    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(invalid("agent_team requires JSON"))
    }

    fn call_json(
        &self,
        context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let plan = parse_plan(&args)?;
        let depth = zk_kit_subagent::current_depth();
        if depth >= zk_kit_subagent::MAX_DEPTH {
            return Err(ToolError::Denied(format!(
                "agent_team depth limit reached (max depth {})",
                zk_kit_subagent::MAX_DEPTH
            )));
        }
        let runner = zk_kit_subagent::runner()
            .ok_or_else(|| ToolError::Denied("agent team runner is not installed".to_owned()))?;
        if let Some(live) = runner.live_count() {
            let available = MAX_WORKERS.saturating_sub(live);
            if plan.tasks.len() > available {
                return Err(ToolError::Denied(format!(
                    "agent team worker cap reached: {live} live async agents, {} requested, cap {MAX_WORKERS}",
                    plan.tasks.len()
                )));
            }
        }
        let mut started: Vec<StartedWorker> = Vec::with_capacity(plan.tasks.len());
        for task in &plan.tasks {
            let prompt = worker_prompt(&plan.objective, task);
            match runner.start(&prompt, context, depth) {
                Ok(id) => started.push(StartedWorker {
                    id,
                    task_id: task.id.clone(),
                }),
                Err(error) => {
                    interrupt_all(&runner, &started);
                    return Err(ToolError::Io(format!(
                        "agent team start failed for task {}: {error}",
                        task.id
                    )));
                }
            }
        }
        let started_at = Instant::now();
        let timeout = Duration::from_millis(plan.timeout_ms);
        let mut reports: Vec<WorkerReport> = Vec::with_capacity(started.len());
        let mut timed_out = false;
        loop {
            reports.clear();
            let mut running = 0_usize;
            for worker in &started {
                let report = report_for(&runner, worker);
                if report.status == "running" {
                    running = running.saturating_add(1);
                }
                reports.push(report);
            }
            if running == 0 {
                break;
            }
            if started_at.elapsed() >= timeout {
                timed_out = true;
                interrupt_all(&runner, &started);
                reports.clear();
                for worker in &started {
                    reports.push(report_for(&runner, worker));
                }
                break;
            }
            std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
        }
        let completed = reports
            .iter()
            .filter(|report| report.status == "done")
            .count();
        let failed = reports
            .iter()
            .filter(|report| report.status == "failed" || report.status == "unknown")
            .count();
        let cancelled = reports
            .iter()
            .filter(|report| report.status == "cancelled")
            .count();
        let total = reports.len();
        let status = if timed_out {
            "timeout"
        } else if failed == 0 && cancelled == 0 && completed == total {
            "done"
        } else if completed == 0 {
            "failed"
        } else {
            "partial"
        };
        let elapsed_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        let workers: Vec<serde_json::Value> = reports.iter().map(worker_json).collect();
        Ok(serde_json::json!({
            "objective": plan.objective,
            "status": status,
            "workers": workers,
            "completed": completed,
            "failed": failed,
            "cancelled": cancelled,
            "total": total,
            "max_workers": plan.max_workers,
            "timeout_ms": plan.timeout_ms,
            "depth": depth,
            "elapsed_ms": elapsed_ms
        })
        .to_string())
    }
}
