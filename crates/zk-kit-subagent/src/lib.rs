//! Subagent runner registry. The agent runtime installs a runner; tool packages use it.
//!
//! @capability kit.subagent
#![forbid(unsafe_code)]
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use zk_contract_tool::TurnContext;

/// One asynchronously started child agent visible to the control tools.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct AsyncAgentStatus {
    /// Stable child id returned by [`SubagentRunner::start`].
    pub id: String,
    /// Delegation depth of the child (1 = direct child of the parent turn).
    pub depth: usize,
    /// `running` | `done` | `failed` | `cancelled`.
    pub status: String,
    /// Original prompt (truncated by the runner).
    pub prompt: String,
}

/// A runner that executes a delegated prompt with the same model/tools.
///
/// Synchronous delegation is mandatory ([`Self::run`] / [`Self::fork`]); async
/// children are optional and degrade honestly: the default implementations
/// report "unsupported" instead of pretending to start work.
pub trait SubagentRunner: Send + Sync {
    /// Run at `depth` (0 = parent); implementation enforces max depth.
    fn run(&self, prompt: &str, context: &TurnContext, depth: usize) -> Result<String, String>;

    /// Fork the current conversation context instead of isolating a child conversation.
    fn fork(&self, prompt: &str, context: &TurnContext, depth: usize) -> Result<String, String> {
        self.run(prompt, context, depth)
    }

    /// Start a background child and return its id without waiting.
    ///
    /// # Errors
    ///
    /// Implementations reject unsupported async execution, depth limits and
    /// concurrency limits with a human-readable message.
    fn start(&self, prompt: &str, context: &TurnContext, depth: usize) -> Result<String, String> {
        let _ = (prompt, context, depth);
        Err("async subagents are not supported by this runner".to_owned())
    }

    /// Current status of one async child, when known.
    fn status(&self, id: &str) -> Option<AsyncAgentStatus> {
        let _ = id;
        None
    }

    /// Final result of one async child once it finished, when known.
    fn result(&self, id: &str) -> Option<Result<String, String>> {
        let _ = id;
        None
    }

    /// Cancel a running async child (already-finished children keep their result).
    ///
    /// # Errors
    ///
    /// Unknown ids are rejected; implementations may also report interrupt
    /// races as errors.
    fn interrupt(&self, id: &str) -> Result<(), String> {
        Err(format!("unknown subagent: {id}"))
    }
}

static RUNNER: OnceLock<Mutex<Option<Arc<dyn SubagentRunner>>>> = OnceLock::new();

/// Install the runner used by the `subagent` tool.
pub fn set_runner(runner: Arc<dyn SubagentRunner>) {
    let slot = RUNNER.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = slot.lock() {
        *guard = Some(runner);
    }
}

/// Resolve the current runner.
#[must_use]
pub fn runner() -> Option<Arc<dyn SubagentRunner>> {
    RUNNER
        .get()
        .and_then(|slot| slot.lock().ok().and_then(|guard| guard.clone()))
}

/// Maximum delegation depth.
pub const MAX_DEPTH: usize = 3;

/// Maximum concurrently running async children per runner.
pub const MAX_ASYNC_AGENTS: usize = 8;

/// One subagent run visible to control tools.
#[derive(Clone, Debug, serde::Serialize)]
pub struct AgentInfo {
    /// Stable child id.
    pub id: String,
    /// Parent conversation id.
    pub parent: String,
    /// Delegation depth.
    pub depth: usize,
    /// running | done | failed
    pub status: String,
    /// Original prompt (truncated by caller).
    pub prompt: String,
}

static AGENTS: OnceLock<Mutex<Vec<AgentInfo>>> = OnceLock::new();
fn agents() -> &'static Mutex<Vec<AgentInfo>> {
    AGENTS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Register a started child.
pub fn register(id: &str, parent: &str, depth: usize, prompt: &str) {
    if let Ok(mut guard) = agents().lock() {
        guard.push(AgentInfo {
            id: id.to_owned(),
            parent: parent.to_owned(),
            depth,
            status: "running".to_owned(),
            prompt: prompt.chars().take(200).collect(),
        });
    }
}

/// Mark a child finished.
pub fn finish(id: &str, ok: bool) {
    if let Ok(mut guard) = agents().lock() {
        if let Some(agent) = guard.iter_mut().find(|agent| agent.id == id) {
            agent.status = if ok {
                "done".to_owned()
            } else {
                "failed".to_owned()
            };
        }
    }
}

/// Mark a child as cancelled (async interrupt).
pub fn mark_cancelled(id: &str) {
    if let Ok(mut guard) = agents().lock() {
        if let Some(agent) = guard.iter_mut().find(|agent| agent.id == id) {
            agent.status = "cancelled".to_owned();
        }
    }
}

/// List known child runs (most recent last).
#[must_use]
pub fn list() -> Vec<AgentInfo> {
    agents()
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

thread_local! {
    static DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Current delegation depth for this thread.
#[must_use]
pub fn current_depth() -> usize {
    DEPTH.with(std::cell::Cell::get)
}

/// Run `f` at `depth` on this thread.
pub fn with_depth<T>(depth: usize, f: impl FnOnce() -> T) -> T {
    DEPTH.with(|cell| {
        let previous = cell.replace(depth);
        let out = f();
        cell.set(previous);
        out
    })
}

static CONTEXT_DEPTHS: OnceLock<Mutex<BTreeMap<String, usize>>> = OnceLock::new();

fn context_depths() -> &'static Mutex<BTreeMap<String, usize>> {
    CONTEXT_DEPTHS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Record the delegation depth of a conversation context.
///
/// A child conversation runs on a fresh thread (and its tool calls on blocking
/// threads), so the thread-local [`current_depth`] cannot carry the delegation
/// depth across the boundary. The agent runtime registers the depth here for the
/// child conversation id and re-applies it with [`with_depth`] inside the tool
/// execution closure, which keeps [`MAX_DEPTH`] enforceable.
pub fn set_context_depth(conversation_id: &str, depth: usize) {
    if let Ok(mut depths) = context_depths().lock() {
        depths.insert(conversation_id.to_owned(), depth);
    }
}

/// Delegation depth recorded for a conversation context (0 when absent).
#[must_use]
pub fn context_depth(conversation_id: &str) -> usize {
    context_depths()
        .lock()
        .ok()
        .and_then(|depths| depths.get(conversation_id).copied())
        .unwrap_or(0)
}

/// Drop the recorded depth for a finished conversation context.
pub fn clear_context_depth(conversation_id: &str) {
    if let Ok(mut depths) = context_depths().lock() {
        depths.remove(conversation_id);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only
    use super::*;

    struct SyncOnly;
    impl SubagentRunner for SyncOnly {
        fn run(
            &self,
            _prompt: &str,
            _context: &TurnContext,
            _depth: usize,
        ) -> Result<String, String> {
            Ok("sync".to_owned())
        }
    }

    #[test]
    fn async_methods_degrade_honestly_by_default() {
        let context = TurnContext {
            conversation_id: "c".to_owned(),
            workspace_id: "w".to_owned(),
            workspace_root: ".".to_owned(),
        };
        let runner = SyncOnly;
        assert!(runner.start("p", &context, 0).is_err());
        assert_eq!(runner.status("missing"), None);
        assert_eq!(runner.result("missing"), None);
        assert!(runner.interrupt("missing").is_err());
        assert_eq!(runner.fork("p", &context, 0), Ok("sync".to_owned()));
    }

    #[test]
    fn async_status_serializes() {
        let status = AsyncAgentStatus {
            id: "subagent-1".to_owned(),
            depth: 1,
            status: "running".to_owned(),
            prompt: "inspect".to_owned(),
        };
        let value = serde_json::to_value(&status).unwrap();
        assert_eq!(value["id"], "subagent-1");
        assert_eq!(value["status"], "running");
    }

    #[test]
    fn context_depth_roundtrip() {
        set_context_depth("kit-depth-context", 2);
        assert_eq!(context_depth("kit-depth-context"), 2);
        clear_context_depth("kit-depth-context");
        assert_eq!(context_depth("kit-depth-context"), 0);
    }
}
