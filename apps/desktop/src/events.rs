#![forbid(unsafe_code)]
//! Event bus shared by the Tauri shell and the HTTP serve carrier.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::broadcast;
use zk_contract_agent::{AgentEvent, EventSink, ToolCallStatus};
use zk_contract_ipc::UiEvent;

/// Broadcast capacity; enough for a turn's deltas even if the UI is briefly slow.
pub const EVENT_BUFFER: usize = 1024;

/// Emit one agent event as a `UiEvent` on the broadcast bus.
pub fn emit_agent(
    bus: &broadcast::Sender<UiEvent>,
    stream_id: &str,
    seq: &AtomicU64,
    event: &AgentEvent,
) {
    let Ok(payload) = serde_json::to_value(event) else {
        return;
    };
    let value = UiEvent {
        stream_id: stream_id.to_owned(),
        seq: seq.fetch_add(1, Ordering::SeqCst),
        kind: "agent".to_owned(),
        payload,
    };
    let _ = bus.send(value);
}

/// Sink that forwards agent events to the bus and appends tool messages to the session store.
pub struct BridgeSink {
    pub bus: broadcast::Sender<UiEvent>,
    pub stream_id: String,
    pub seq: Arc<AtomicU64>,
    pub cancelled: Arc<AtomicBool>,
    pub sessions: Arc<dyn zk_contract_session::SessionStore>,
    pub conversation_id: String,
    /// Turn id observed from `TurnStarted` (tool rows use it as their owner).
    turn_id: std::sync::Mutex<String>,
    /// Set once `TurnCancelled` was observed (assistant status source).
    turn_cancelled: AtomicBool,
}

impl BridgeSink {
    /// Create a sink with an empty turn owner.
    #[must_use]
    pub fn new(
        bus: broadcast::Sender<UiEvent>,
        stream_id: String,
        seq: Arc<AtomicU64>,
        cancelled: Arc<AtomicBool>,
        sessions: Arc<dyn zk_contract_session::SessionStore>,
        conversation_id: String,
    ) -> Self {
        Self {
            bus,
            stream_id,
            seq,
            cancelled,
            sessions,
            conversation_id,
            turn_id: std::sync::Mutex::new(String::new()),
            turn_cancelled: AtomicBool::new(false),
        }
    }

    /// Turn id currently observed on this stream (empty before `TurnStarted`).
    #[must_use]
    pub fn turn_id(&self) -> String {
        self.turn_id
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// Whether this stream observed a `TurnCancelled` event.
    #[must_use]
    pub fn turn_cancelled(&self) -> bool {
        self.turn_cancelled.load(Ordering::SeqCst)
    }

    fn persist_tool_row(&self, name: &str, args: &str, result: &str, status: ToolCallStatus) {
        let content = serde_json::json!({
            "name": name,
            "args": args,
            "status": status,
            "result": result,
        })
        .to_string();
        let message = zk_contract_session::Message {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: self.conversation_id.clone(),
            role: "tool".to_owned(),
            content,
            created_at_ms: now_ms(),
            status: "done".to_owned(),
            usage: None,
            turn_id: self.turn_id(),
        };
        let _ = self.sessions.append_message(&message);
    }
}

impl EventSink for BridgeSink {
    fn emit(&self, event: AgentEvent) {
        // Terminal tool rows are persisted *before* the cancelled short-circuit:
        // a cancelled turn must still keep its `cancelled` tool row (P1-R3).
        if let AgentEvent::ToolCallFinished {
            name,
            result,
            args,
            status,
        } = &event
        {
            self.persist_tool_row(name, args, result, *status);
        }
        let cancelled_now = self.cancelled.load(Ordering::SeqCst);
        match &event {
            AgentEvent::TurnStarted { turn_id } => {
                if let Ok(mut guard) = self.turn_id.lock() {
                    *guard = turn_id.clone();
                }
            }
            AgentEvent::TurnCancelled => {
                self.turn_cancelled.store(true, Ordering::SeqCst);
            }
            _ => {}
        }
        let is_terminal = matches!(
            event,
            AgentEvent::TurnCompleted { .. }
                | AgentEvent::TurnCancelled
                | AgentEvent::ToolCallFinished { .. }
                | AgentEvent::Error { .. }
        );
        // Cancelled streams only keep terminal events on the bus; tool/output
        // deltas are stale by then.
        if cancelled_now && !is_terminal {
            return;
        }
        emit_agent(&self.bus, &self.stream_id, &self.seq, &event);
    }
}

/// Milliseconds since UNIX epoch.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only
    use super::*;
    use std::sync::Mutex;
    use zk_contract_session::{
        Conversation, Message, RemoveWorkspaceResult, SessionError, SessionStore, Workspace,
        WorkspaceStore,
    };

    #[derive(Default)]
    struct FakeStore(Mutex<Vec<Message>>);

    impl WorkspaceStore for FakeStore {
        fn list_workspaces(&self) -> Result<Vec<Workspace>, SessionError> {
            Ok(Vec::new())
        }
        fn create_workspace(&self, _path: &str, _title: &str) -> Result<Workspace, SessionError> {
            Err(SessionError::Invalid("unused".to_owned()))
        }
        fn rename_workspace(
            &self,
            _workspace_id: &str,
            _title: &str,
        ) -> Result<Workspace, SessionError> {
            Err(SessionError::Invalid("unused".to_owned()))
        }
        fn set_default_workspace(
            &self,
            _workspace_id: &str,
        ) -> Result<Vec<Workspace>, SessionError> {
            Ok(Vec::new())
        }
        fn remove_workspace(
            &self,
            _workspace_id: &str,
            _reassign_to: Option<&str>,
            _confirm: bool,
        ) -> Result<RemoveWorkspaceResult, SessionError> {
            Err(SessionError::Invalid("unused".to_owned()))
        }
    }

    impl SessionStore for FakeStore {
        fn create_conversation_in(
            &self,
            _workspace_id: &str,
            _title: &str,
        ) -> Result<Conversation, SessionError> {
            Err(SessionError::Invalid("unused".to_owned()))
        }
        fn list_conversations_in(
            &self,
            _workspace_id: &str,
        ) -> Result<Vec<Conversation>, SessionError> {
            Ok(Vec::new())
        }
        fn append_message(&self, message: &Message) -> Result<(), SessionError> {
            self.0.lock().unwrap().push(message.clone());
            Ok(())
        }
        fn messages_in(
            &self,
            _workspace_id: &str,
            _conversation_id: &str,
        ) -> Result<Vec<Message>, SessionError> {
            Ok(self.0.lock().unwrap().clone())
        }
    }

    fn sink(
        cancelled: bool,
    ) -> (
        Arc<BridgeSink>,
        Arc<FakeStore>,
        broadcast::Receiver<UiEvent>,
    ) {
        let (bus, rx) = broadcast::channel(EVENT_BUFFER);
        let store = Arc::new(FakeStore::default());
        let bridge = Arc::new(BridgeSink::new(
            bus,
            "s1".to_owned(),
            Arc::new(AtomicU64::new(1)),
            Arc::new(AtomicBool::new(cancelled)),
            store.clone(),
            "c1".to_owned(),
        ));
        (bridge, store, rx)
    }

    #[test]
    fn terminal_tool_row_is_persisted_before_cancelled_short_circuit() {
        let (bridge, store, mut rx) = sink(true);
        bridge.emit(AgentEvent::TurnStarted {
            turn_id: "turn-1".to_owned(),
        });
        bridge.emit(AgentEvent::ToolCallFinished {
            name: "fs.write".to_owned(),
            result: "wrote".to_owned(),
            args: "{\"path\":\"a.txt\"}".to_owned(),
            status: ToolCallStatus::Cancelled,
        });
        bridge.emit(AgentEvent::TextDelta {
            delta: "stale".to_owned(),
        });
        let rows = store.0.lock().unwrap().clone();
        assert_eq!(rows.len(), 1, "cancelled tool row must be persisted");
        assert_eq!(rows[0].role, "tool");
        assert_eq!(rows[0].turn_id, "turn-1");
        assert!(rows[0].content.contains("\"status\":\"cancelled\""));
        assert_eq!(bridge.turn_id(), "turn-1");
        let mut kinds: Vec<String> = Vec::new();
        while let Ok(event) = rx.try_recv() {
            if let Ok(value) = serde_json::to_value(&event.payload) {
                let kind = value
                    .get("type")
                    .and_then(|kind| kind.as_str())
                    .unwrap_or("")
                    .to_owned();
                kinds.push(kind);
            }
        }
        assert!(
            kinds.iter().any(|kind| kind == "tool_call_finished"),
            "{kinds:?}"
        );
        assert!(!kinds.iter().any(|kind| kind == "text_delta"), "{kinds:?}");
    }

    #[test]
    fn turn_cancelled_is_the_assistant_status_source() {
        let (bridge, _store, mut rx) = sink(false);
        assert!(!bridge.turn_cancelled());
        bridge.emit(AgentEvent::TurnCancelled);
        assert!(bridge.turn_cancelled());
        let event = rx.try_recv().unwrap();
        let value = serde_json::to_value(&event.payload).unwrap();
        assert_eq!(
            value.get("type").and_then(|kind| kind.as_str()),
            Some("turn_cancelled")
        );
    }
}
