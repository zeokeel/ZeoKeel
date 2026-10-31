#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use zk_contract_tool::{Tool, ToolError, TurnContext};
use zk_kit_subagent::{AsyncAgentStatus, SubagentRunner};

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    match TEST_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Done,
    Mixed,
    Never,
}

struct Child {
    prompt: String,
    polls: usize,
    cancelled: bool,
}

/// Deterministic in-process worker runner (no network, no model).
struct FakeRunner {
    mode: Mode,
    live: usize,
    fail_on_start: Option<usize>,
    starts: AtomicUsize,
    next: AtomicUsize,
    children: Mutex<HashMap<String, Child>>,
    prompts: Mutex<Vec<String>>,
    depths: Mutex<Vec<usize>>,
    interrupted: Mutex<Vec<String>>,
}

impl FakeRunner {
    fn new(mode: Mode) -> Self {
        Self {
            mode,
            live: 0,
            fail_on_start: None,
            starts: AtomicUsize::new(0),
            next: AtomicUsize::new(0),
            children: Mutex::new(HashMap::new()),
            prompts: Mutex::new(Vec::new()),
            depths: Mutex::new(Vec::new()),
            interrupted: Mutex::new(Vec::new()),
        }
    }

    fn with_live(mut self, live: usize) -> Self {
        self.live = live;
        self
    }

    fn with_start_failure(mut self, at: usize) -> Self {
        self.fail_on_start = Some(at);
        self
    }

    fn terminal(&self, child: &Child) -> Option<&'static str> {
        if child.polls < 2 {
            return None;
        }
        match self.mode {
            Mode::Done => Some("done"),
            Mode::Mixed => Some(if child.prompt.contains("FAIL") {
                "failed"
            } else {
                "done"
            }),
            Mode::Never => None,
        }
    }
}

impl SubagentRunner for FakeRunner {
    fn run(&self, prompt: &str, _context: &TurnContext, _depth: usize) -> Result<String, String> {
        Ok(prompt.to_owned())
    }

    fn start(&self, prompt: &str, _context: &TurnContext, depth: usize) -> Result<String, String> {
        let index = self.starts.fetch_add(1, Ordering::SeqCst);
        if self.fail_on_start == Some(index) {
            return Err("injected start failure".to_owned());
        }
        let id = format!("fake-{}", self.next.fetch_add(1, Ordering::SeqCst));
        self.children.lock().unwrap().insert(
            id.clone(),
            Child {
                prompt: prompt.to_owned(),
                polls: 0,
                cancelled: false,
            },
        );
        self.prompts.lock().unwrap().push(prompt.to_owned());
        self.depths.lock().unwrap().push(depth);
        Ok(id)
    }

    fn status(&self, id: &str) -> Option<AsyncAgentStatus> {
        let mut children = self.children.lock().ok()?;
        let child = children.get_mut(id)?;
        child.polls = child.polls.saturating_add(1);
        let status = if child.cancelled {
            "cancelled"
        } else if self.mode == Mode::Never {
            "running"
        } else {
            self.terminal(child).unwrap_or("running")
        };
        Some(AsyncAgentStatus {
            id: id.to_owned(),
            depth: 1,
            status: status.to_owned(),
            prompt: child.prompt.chars().take(80).collect(),
        })
    }

    fn result(&self, id: &str) -> Option<Result<String, String>> {
        let children = self.children.lock().ok()?;
        let child = children.get(id)?;
        if child.cancelled {
            return Some(Err("subagent cancelled".to_owned()));
        }
        Some(match self.terminal(child)? {
            "done" => Ok(format!("result:{id}")),
            _ => Err("injected worker failure".to_owned()),
        })
    }

    fn interrupt(&self, id: &str) -> Result<(), String> {
        let mut children = self
            .children
            .lock()
            .map_err(|_| "poisoned registry".to_owned())?;
        let child = children
            .get_mut(id)
            .ok_or_else(|| format!("unknown subagent: {id}"))?;
        if child.cancelled || self.terminal(child).is_some() {
            return Err(format!("subagent {id} already finished"));
        }
        child.cancelled = true;
        drop(children);
        self.interrupted.lock().unwrap().push(id.to_owned());
        Ok(())
    }

    fn live_count(&self) -> Option<usize> {
        Some(self.live)
    }
}

fn context() -> TurnContext {
    TurnContext {
        conversation_id: "conversation".to_owned(),
        workspace_id: "workspace".to_owned(),
        workspace_root: ".".to_owned(),
    }
}

fn call(args: serde_json::Value) -> Result<String, ToolError> {
    zk_tool_agent_team::tool().call_json(&context(), args)
}

fn parse(text: &str) -> serde_json::Value {
    serde_json::from_str(text).unwrap()
}

fn install(runner: Arc<FakeRunner>) -> Arc<FakeRunner> {
    zk_kit_subagent::set_runner(runner.clone());
    runner
}

#[test]
fn dispatches_workers_and_aggregates_success() {
    let _guard = lock();
    let runner = install(Arc::new(FakeRunner::new(Mode::Done)));
    let out = call(serde_json::json!({
        "objective": "ship the release",
        "tasks": [{"id": "a", "prompt": "one"}, {"prompt": "two"}]
    }))
    .unwrap();
    let value = parse(&out);
    assert_eq!(value["status"], "done");
    assert_eq!(value["completed"], 2);
    assert_eq!(value["failed"], 0);
    assert_eq!(value["total"], 2);
    assert_eq!(value["workers"][0]["task"], "a");
    assert_eq!(value["workers"][1]["task"], "worker-2");
    assert_eq!(value["workers"][1]["status"], "done");
    assert!(value["workers"][0]["result"]
        .as_str()
        .unwrap_or("")
        .starts_with("result:fake-"));
    let prompts = runner.prompts.lock().unwrap();
    assert_eq!(prompts.len(), 2);
    assert!(prompts[0].contains("ship the release"));
    assert!(prompts[0].contains("one"));
}

#[test]
fn partial_failure_is_reported_per_worker() {
    let _guard = lock();
    install(Arc::new(FakeRunner::new(Mode::Mixed)));
    let out = call(serde_json::json!({
        "objective": "mixed team",
        "tasks": [{"id": "ok", "prompt": "do good"}, {"id": "bad", "prompt": "FAIL please"}]
    }))
    .unwrap();
    let value = parse(&out);
    assert_eq!(value["status"], "partial");
    assert_eq!(value["completed"], 1);
    assert_eq!(value["failed"], 1);
    assert_eq!(value["workers"][1]["status"], "failed");
    assert!(value["workers"][1]["error"]
        .as_str()
        .unwrap_or("")
        .contains("injected worker failure"));
}

#[test]
fn all_workers_failed_reports_failed() {
    let _guard = lock();
    install(Arc::new(FakeRunner::new(Mode::Mixed)));
    let out = call(serde_json::json!({
        "objective": "all fail",
        "tasks": [{"prompt": "FAIL one"}, {"prompt": "FAIL two"}]
    }))
    .unwrap();
    assert_eq!(parse(&out)["status"], "failed");
}

#[test]
fn timeout_interrupts_running_workers() {
    let _guard = lock();
    let runner = install(Arc::new(FakeRunner::new(Mode::Never)));
    let out = call(serde_json::json!({
        "objective": "never finishes",
        "tasks": [{"prompt": "one"}, {"prompt": "two"}],
        "timeout_ms": 100
    }))
    .unwrap();
    let value = parse(&out);
    assert_eq!(value["status"], "timeout");
    assert_eq!(value["cancelled"], 2);
    assert_eq!(value["workers"][0]["status"], "cancelled");
    assert_eq!(runner.interrupted.lock().unwrap().len(), 2);
}

#[test]
fn cap_is_rejected_before_dispatch() {
    let _guard = lock();
    let runner = install(Arc::new(
        FakeRunner::new(Mode::Done).with_live(zk_kit_subagent::MAX_ASYNC_AGENTS),
    ));
    let error = call(serde_json::json!({
        "objective": "cap",
        "tasks": [{"prompt": "one"}]
    }))
    .unwrap_err();
    assert!(matches!(error, ToolError::Denied(ref message) if message.contains("cap reached")));
    assert!(runner.prompts.lock().unwrap().is_empty());
}

#[test]
fn start_failure_interrupts_already_started_workers() {
    let _guard = lock();
    let runner = install(Arc::new(FakeRunner::new(Mode::Done).with_start_failure(1)));
    let error = call(serde_json::json!({
        "objective": "start failure",
        "tasks": [{"prompt": "one"}, {"prompt": "two"}]
    }))
    .unwrap_err();
    assert!(
        matches!(error, ToolError::Io(ref message) if message.contains("start failed for task worker-2"))
    );
    assert_eq!(runner.interrupted.lock().unwrap().len(), 1);
}

#[test]
fn depth_limit_is_enforced_and_child_depth_passes_through() {
    let _guard = lock();
    let runner = install(Arc::new(FakeRunner::new(Mode::Done)));
    let plan = serde_json::json!({
        "objective": "depth",
        "tasks": [{"prompt": "one"}]
    });
    let error =
        zk_kit_subagent::with_depth(zk_kit_subagent::MAX_DEPTH, || call(plan.clone())).unwrap_err();
    assert!(matches!(error, ToolError::Denied(ref message) if message.contains("depth limit")));
    assert!(
        zk_kit_subagent::with_depth(zk_kit_subagent::MAX_DEPTH.saturating_sub(1), || call(plan))
            .is_ok()
    );
    assert_eq!(
        runner.depths.lock().unwrap().as_slice(),
        [zk_kit_subagent::MAX_DEPTH.saturating_sub(1)]
    );
}

#[test]
fn invalid_arguments_are_rejected() {
    let _guard = lock();
    install(Arc::new(FakeRunner::new(Mode::Done)));
    let contexts: [serde_json::Value; 7] = [
        serde_json::json!({"objective": "x", "tasks": []}),
        serde_json::json!({"objective": "", "tasks": [{"prompt": "one"}]}),
        serde_json::json!({"objective": "x", "tasks": [{"prompt": ""}]}),
        serde_json::json!({"objective": "x", "tasks": [{"prompt": "one"}, {"prompt": "two"}], "max_workers": 1}),
        serde_json::json!({"objective": "x", "tasks": [{"prompt": "one"}], "max_workers": 0}),
        serde_json::json!({"objective": "x", "tasks": [{"prompt": "one"}], "max_workers": 9}),
        serde_json::json!({"objective": "x", "tasks": [{"id": "a", "prompt": "one"}, {"id": "a", "prompt": "two"}]}),
    ];
    for args in contexts {
        assert!(matches!(call(args), Err(ToolError::InvalidArgs(_))));
    }
    let too_many: Vec<serde_json::Value> = (0..9)
        .map(|index| serde_json::json!({"prompt": format!("task {index}")}))
        .collect();
    assert!(matches!(
        call(serde_json::json!({"objective": "x", "tasks": too_many})),
        Err(ToolError::InvalidArgs(_))
    ));
    assert!(matches!(
        zk_tool_agent_team::tool().call("{}"),
        Err(ToolError::InvalidArgs(_))
    ));
}

#[test]
fn spec_declares_bounded_team_schema() {
    let spec = zk_tool_agent_team::tool().spec();
    assert_eq!(spec.name, "agent_team");
    assert!(!spec.mutating);
    assert_eq!(
        spec.parameters["required"],
        serde_json::json!(["objective", "tasks"])
    );
    assert_eq!(spec.parameters["properties"]["tasks"]["maxItems"], 8);
    assert_eq!(spec.parameters["properties"]["max_workers"]["maximum"], 8);
}
