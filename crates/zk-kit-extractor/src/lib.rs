//! Structured extraction runner registry (rig extractor).
//!
//! @capability kit.extractor
#![forbid(unsafe_code)]
use std::sync::{Arc, Mutex, OnceLock};
use zk_contract_tool::TurnContext;

/// Runner that extracts structured JSON from text using an agent model.
pub trait ExtractRunner: Send + Sync {
    /// Extract a JSON object/array from `text`.
    fn extract(&self, text: &str, context: &TurnContext) -> Result<serde_json::Value, String>;
}

static RUNNER: OnceLock<Mutex<Option<Arc<dyn ExtractRunner>>>> = OnceLock::new();
fn slot() -> &'static Mutex<Option<Arc<dyn ExtractRunner>>> {
    RUNNER.get_or_init(|| Mutex::new(None))
}
/// Install the extractor runner.
pub fn set_runner(runner: Arc<dyn ExtractRunner>) {
    if let Ok(mut guard) = slot().lock() {
        *guard = Some(runner);
    }
}
/// Resolve the extractor runner.
#[must_use]
pub fn runner() -> Option<Arc<dyn ExtractRunner>> {
    slot().lock().ok().and_then(|guard| guard.clone())
}
