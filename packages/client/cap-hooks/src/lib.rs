//! Agent lifecycle hooks capability (`agent.hooks`).
//!
//! The shipped `zk-agent-rig` runtime implements turn/tool/approval/usage hook
//! events and accepts a `HookSink` (plus optional JSONL recording). This package
//! reports the capability **dynamically**: available by default (the shipped
//! agent supports hooks), false when `ZEOKEEL_AGENT_HOOKS_ENABLED=0` or when a
//! host explicitly disables it via [`set_available`]. `CAPABILITY_UNAVAILABLE`
//! is the closed error code hosts surface when [`unavailable_reason`] is set.
#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, RwLock};

/// Closed unavailability code.
pub const UNAVAILABLE: &str = "CAPABILITY_UNAVAILABLE";

/// Whether this build implements the hooks capability (the shipped agent does).
pub const HOOKS_IMPLEMENTED: bool = true;

static AVAILABLE: AtomicBool = AtomicBool::new(HOOKS_IMPLEMENTED);
static REASON: OnceLock<RwLock<Option<String>>> = OnceLock::new();

fn reason_slot() -> &'static RwLock<Option<String>> {
    REASON.get_or_init(|| RwLock::new(None))
}

/// Host/desktop override: declare hooks available or unavailable with a reason.
pub fn set_available(available: bool, reason: impl Into<String>) {
    AVAILABLE.store(available, Ordering::SeqCst);
    if let Ok(mut slot) = reason_slot().write() {
        *slot = if available { None } else { Some(reason.into()) };
    }
}

/// Whether lifecycle hooks can be observed right now.
#[must_use]
pub fn available() -> bool {
    if !HOOKS_IMPLEMENTED {
        return false;
    }
    if std::env::var("ZEOKEEL_AGENT_HOOKS_ENABLED").ok().as_deref() == Some("0") {
        return false;
    }
    AVAILABLE.load(Ordering::SeqCst)
}

/// Why hooks are unavailable, or `None` when available.
#[must_use]
pub fn unavailable_reason() -> Option<String> {
    if HOOKS_IMPLEMENTED
        && std::env::var("ZEOKEEL_AGENT_HOOKS_ENABLED").ok().as_deref() == Some("0")
    {
        return Some("hooks disabled by ZEOKEEL_AGENT_HOOKS_ENABLED=0".to_owned());
    }
    if AVAILABLE.load(Ordering::SeqCst) {
        return None;
    }
    reason_slot()
        .read()
        .ok()
        .and_then(|slot| slot.clone())
        .or_else(|| Some("hooks disabled by the host".to_owned()))
}
