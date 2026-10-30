//! Process-wide approval gate registry shared by the agent and desktop IPC.
//!
//! @capability kit.approval
#![forbid(unsafe_code)]
use std::sync::{Arc, Mutex, OnceLock};
use zk_contract_tool::{ApprovalGate, DenyAll};

static GATE: OnceLock<Mutex<Arc<dyn ApprovalGate>>> = OnceLock::new();

/// Install the gate used by agent tool calls (desktop broker).
pub fn set_global_gate(gate: Arc<dyn ApprovalGate>) {
    let slot = GATE.get_or_init(|| Mutex::new(Arc::new(DenyAll)));
    if let Ok(mut guard) = slot.lock() { *guard = gate; }
}

/// Resolve the current gate at call time.
#[must_use]
pub fn current() -> Arc<dyn ApprovalGate> {
    GATE.get().and_then(|slot| slot.lock().ok().map(|guard| Arc::clone(&guard))).unwrap_or_else(|| Arc::new(DenyAll))
}

/// Late-bound gate: install after the agent has been composed.
pub struct GlobalGate;
impl ApprovalGate for GlobalGate {
    fn decide(&self, approval_id: &str, tool: &str, args: &serde_json::Value, reason: &str) -> bool {
        current().decide(approval_id, tool, args, reason)
    }
}
