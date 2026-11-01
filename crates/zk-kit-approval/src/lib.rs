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
    if let Ok(mut guard) = slot.lock() {
        *guard = gate;
    }
}

/// Resolve the current gate at call time.
#[must_use]
pub fn current() -> Arc<dyn ApprovalGate> {
    GATE.get()
        .and_then(|slot| slot.lock().ok().map(|guard| Arc::clone(&guard)))
        .unwrap_or_else(|| Arc::new(DenyAll))
}

/// Late-bound gate: install after the agent has been composed.
pub struct GlobalGate;
impl ApprovalGate for GlobalGate {
    fn decide(
        &self,
        approval_id: &str,
        tool: &str,
        args: &serde_json::Value,
        reason: &str,
    ) -> bool {
        current().decide(approval_id, tool, args, reason)
    }

    fn decide_scoped(
        &self,
        approval_id: &str,
        tool: &str,
        args: &serde_json::Value,
        reason: &str,
    ) -> zk_contract_tool::ApprovalDecision {
        // Typed decisions must reach the installed gate unchanged; the trait's
        // default would collapse allow_session/cancel into allow_once/reject.
        current().decide_scoped(approval_id, tool, args, reason)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
    use super::*;
    use zk_contract_tool::ApprovalDecision;

    struct Fixed(ApprovalDecision);

    impl ApprovalGate for Fixed {
        fn decide(
            &self,
            _approval_id: &str,
            _tool: &str,
            _args: &serde_json::Value,
            _reason: &str,
        ) -> bool {
            self.0.is_allowed()
        }
        fn decide_scoped(
            &self,
            _approval_id: &str,
            _tool: &str,
            _args: &serde_json::Value,
            _reason: &str,
        ) -> ApprovalDecision {
            self.0
        }
    }

    #[test]
    fn global_gate_forwards_typed_decisions() {
        set_global_gate(Arc::new(Fixed(ApprovalDecision::AllowSession)));
        assert_eq!(
            GlobalGate.decide_scoped("a", "fs.write", &serde_json::json!({}), "r"),
            ApprovalDecision::AllowSession
        );
        set_global_gate(Arc::new(Fixed(ApprovalDecision::Cancel)));
        assert_eq!(
            GlobalGate.decide_scoped("a", "fs.write", &serde_json::json!({}), "r"),
            ApprovalDecision::Cancel
        );
        set_global_gate(Arc::new(zk_contract_tool::DenyAll));
    }
}
