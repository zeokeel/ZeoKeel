//! Turn-scoped event sink for tools that need to emit during a run.
//!
//! @capability kit.events
#![forbid(unsafe_code)]
use std::sync::{Arc, Mutex, OnceLock};
use zk_contract_agent::{AgentEvent, EventSink};

static SINK: OnceLock<Mutex<Option<Arc<dyn EventSink>>>> = OnceLock::new();
fn slot() -> &'static Mutex<Option<Arc<dyn EventSink>>> { SINK.get_or_init(|| Mutex::new(None)) }

/// Install the active turn sink (agent runtime).
pub fn set_sink(sink: Arc<dyn EventSink>) { if let Ok(mut guard) = slot().lock() { *guard = Some(sink); } }

/// Clear the active sink at turn end.
pub fn clear() { if let Ok(mut guard) = slot().lock() { *guard = None; } }

/// Emit an event through the active sink; no-op outside a turn.
pub fn emit(event: AgentEvent) {
    if let Ok(guard) = slot().lock() { if let Some(sink) = guard.as_ref() { sink.emit(event); } }
}
