//! Subagent runner registry. The agent runtime installs a runner; tool packages use it.
//!
//! @capability kit.subagent
#![forbid(unsafe_code)]
use std::sync::{Arc, Mutex, OnceLock};

use zk_contract_tool::TurnContext;

/// A runner that executes a delegated prompt with the same model/tools.
pub trait SubagentRunner: Send + Sync {
    /// Run at `depth` (0 = parent); implementation enforces max depth.
    fn run(&self, prompt: &str, context: &TurnContext, depth: usize) -> Result<String, String>;
}

static RUNNER: OnceLock<Mutex<Option<Arc<dyn SubagentRunner>>>> = OnceLock::new();

/// Install the runner used by the `subagent` tool.
pub fn set_runner(runner: Arc<dyn SubagentRunner>) {
    let slot = RUNNER.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = slot.lock() { *guard = Some(runner); }
}

/// Resolve the current runner.
#[must_use]
pub fn runner() -> Option<Arc<dyn SubagentRunner>> {
    RUNNER.get().and_then(|slot| slot.lock().ok().and_then(|guard| guard.clone()))
}

/// Maximum delegation depth.
pub const MAX_DEPTH: usize = 3;

thread_local! {
    static DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Current delegation depth for this thread.
#[must_use]
pub fn current_depth() -> usize { DEPTH.with(std::cell::Cell::get) }

/// Run `f` at `depth` on this thread.
pub fn with_depth<T>(depth: usize, f: impl FnOnce() -> T) -> T { DEPTH.with(|cell| { let previous = cell.replace(depth); let out = f(); cell.set(previous); out }) }
