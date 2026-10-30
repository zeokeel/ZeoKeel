#![forbid(unsafe_code)]
//! Event bus shared by the Tauri shell and the HTTP serve carrier.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::broadcast;
use zk_contract_agent::{AgentEvent, EventSink};
use zk_contract_ipc::UiEvent;

/// Broadcast capacity; enough for a turn's deltas even if the UI is briefly slow.
pub const EVENT_BUFFER: usize = 1024;

/// Emit one agent event as a `UiEvent` on the broadcast bus.
pub fn emit_agent(bus: &broadcast::Sender<UiEvent>, stream_id: &str, seq: &AtomicU64, event: &AgentEvent) {
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

pub fn emit_kind(bus: &broadcast::Sender<UiEvent>, stream_id: &str, seq: &AtomicU64, kind: &str, payload: Value) {
    let value = UiEvent {
        stream_id: stream_id.to_owned(),
        seq: seq.fetch_add(1, Ordering::SeqCst),
        kind: kind.to_owned(),
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
}

impl EventSink for BridgeSink {
    fn emit(&self, event: AgentEvent) {
        if self.cancelled.load(Ordering::SeqCst) && !matches!(event, AgentEvent::TurnCancelled) {
            return;
        }
        if let AgentEvent::ToolCallFinished { name, result } = &event {
            let content = serde_json::json!({ "name": name, "result": result }).to_string();
            let message = zk_contract_session::Message {
                id: uuid::Uuid::new_v4().to_string(),
                conversation_id: self.conversation_id.clone(),
                role: "tool".to_owned(),
                content,
                created_at_ms: now_ms(),
            };
            let _ = self.sessions.append_message(&message);
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
