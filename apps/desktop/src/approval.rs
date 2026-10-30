//! Desktop approval broker: agent threads block until UI calls `approval.decide`.
#![forbid(unsafe_code)]
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use zk_contract_tool::ApprovalGate;

/// Routes approval decisions from IPC to waiting agent tool calls.
pub struct ApprovalBroker {
    waiters: Mutex<HashMap<String, mpsc::Sender<bool>>>,
    seq: AtomicU64,
    timeout: Duration,
}

impl Default for ApprovalBroker {
    fn default() -> Self {
        Self {
            waiters: Mutex::new(HashMap::new()),
            seq: AtomicU64::new(0),
            timeout: Duration::from_millis(
                std::env::var("ZEOKEEL_APPROVAL_TIMEOUT_MS")
                    .ok()
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(300_000)
                    .clamp(1_000, 1_800_000),
            ),
        }
    }
}

impl ApprovalBroker {
    /// Create an empty broker.
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Broker with an explicit timeout (tests).
    #[must_use]
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn with_timeout(timeout: Duration) -> Arc<Self> {
        Arc::new(Self {
            waiters: Mutex::new(HashMap::new()),
            seq: AtomicU64::new(0),
            timeout,
        })
    }

    /// Resolve a pending request; returns false if unknown/expired.
    pub fn resolve(&self, approval_id: &str, allow: bool) -> bool {
        self.waiters
            .lock()
            .ok()
            .and_then(|mut guard| guard.remove(approval_id))
            .is_some_and(|sender| sender.send(allow).is_ok())
    }

    /// Number of pending requests (tests).
    #[must_use]
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn pending(&self) -> usize {
        self.waiters.lock().map(|guard| guard.len()).unwrap_or(0)
    }
}

impl ApprovalGate for ApprovalBroker {
    fn decide(
        &self,
        approval_id: &str,
        _tool: &str,
        _args: &serde_json::Value,
        _reason: &str,
    ) -> bool {
        let (tx, rx) = mpsc::channel();
        if let Ok(mut guard) = self.waiters.lock() {
            guard.insert(approval_id.to_owned(), tx);
        }
        let _ = self.seq.fetch_add(1, Ordering::SeqCst);
        rx.recv_timeout(self.timeout).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
    use super::*;

    #[test]
    fn broker_times_out_fail_closed() {
        let broker = ApprovalBroker::with_timeout(Duration::from_millis(20));
        assert!(!broker.decide("missing", "bash", &serde_json::json!({}), "mutating"));
    }

    #[test]
    fn broker_resolves_waiting_tool() {
        let broker = ApprovalBroker::new();
        let wait = Arc::clone(&broker);
        let handle = std::thread::spawn(move || {
            wait.decide("a1", "bash", &serde_json::json!({}), "mutating")
        });
        std::thread::sleep(Duration::from_millis(20));
        assert!(broker.resolve("a1", true));
        assert!(handle.join().unwrap());
        assert_eq!(broker.pending(), 0);
    }
}
