//! Subagent runner registry. The agent runtime installs a runner; tool packages use it.
//!
//! @capability kit.subagent
#![forbid(unsafe_code)]
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use zk_contract_tool::TurnContext;

/// A runner that executes a delegated prompt with the same model/tools.
pub trait SubagentRunner: Send + Sync {
    /// Run at `depth` (0 = parent); implementation enforces max depth.
    fn run(&self, prompt: &str, context: &TurnContext, depth: usize) -> Result<String, String>;

    /// Fork the current conversation context instead of isolating a child conversation.
    fn fork(&self, prompt: &str, context: &TurnContext, depth: usize) -> Result<String, String> {
        self.run(prompt, context, depth)
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
    #![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
    use super::*;

    #[test]
    fn context_depth_roundtrip() {
        set_context_depth("kit-depth-context", 2);
        assert_eq!(context_depth("kit-depth-context"), 2);
        clear_context_depth("kit-depth-context");
        assert_eq!(context_depth("kit-depth-context"), 0);
    }
}
