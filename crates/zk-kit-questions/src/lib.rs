//! Blocking user-question broker (UI answers through IPC).
//!
//! @capability kit.questions
#![forbid(unsafe_code)]
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// Answers payload (array of question answers) or an error.
pub type Answers = serde_json::Value;

#[derive(Default)]
pub struct QuestionBroker { waiters: Mutex<HashMap<String, mpsc::Sender<Answers>>> }

impl QuestionBroker {
    /// Create a broker.
    #[must_use] pub fn new() -> Arc<Self> { Arc::new(Self::default()) }
    /// Answer a pending question; false if unknown/expired.
    pub fn answer(&self, id: &str, answers: Answers) -> bool {
        self.waiters.lock().ok().and_then(|mut guard| guard.remove(id)).is_some_and(|sender| sender.send(answers).is_ok())
    }
    /// Block until answered/timeout (returns Null on timeout).
    pub fn ask(&self, id: &str, timeout: Duration) -> Answers {
        let (tx, rx) = mpsc::channel();
        if let Ok(mut guard) = self.waiters.lock() { guard.insert(id.to_owned(), tx); }
        rx.recv_timeout(timeout).unwrap_or(serde_json::Value::Null)
    }
    /// Pending count (tests).
    #[must_use] pub fn pending(&self) -> usize { self.waiters.lock().map(|guard| guard.len()).unwrap_or(0) }
}

static BROKER: OnceLock<Arc<QuestionBroker>> = OnceLock::new();
/// Install the process-wide broker.
pub fn set_global(broker: Arc<QuestionBroker>) { let _ = BROKER.set(broker); }
/// Global broker, if installed.
#[must_use] pub fn global() -> Option<Arc<QuestionBroker>> { BROKER.get().cloned() }
