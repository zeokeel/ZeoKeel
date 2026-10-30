//! transcription capability (transcription).
//!
//! The capability is **honestly unavailable in this build**: rig transcription provider wiring is not
//! enabled/configured, so [`available`] is false and [`unavailable_reason`]
//! explains why. [`UNAVAILABLE`] is the closed error code hosts surface instead
//! of silently dropping the feature. When a provider/feature lands, a host can
//! flip [`set_available`] after configuring it.
#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicBool, Ordering};

/// Closed unavailability code.
pub const UNAVAILABLE: &str = "CAPABILITY_UNAVAILABLE";

/// Whether the underlying rig feature/provider is compiled and configured.
pub const FEATURE_AVAILABLE: bool = false;

static AVAILABLE: AtomicBool = AtomicBool::new(FEATURE_AVAILABLE);

/// Host/provider hook: declare the capability available after configuring a provider.
pub fn set_available(available: bool) {
    AVAILABLE.store(available && FEATURE_AVAILABLE, Ordering::SeqCst);
}

/// Whether transcription can be used right now.
#[must_use]
pub fn available() -> bool {
    AVAILABLE.load(Ordering::SeqCst)
}

/// Why transcription is unavailable, or `None` when available.
#[must_use]
pub fn unavailable_reason() -> Option<&'static str> {
    if available() {
        return None;
    }
    Some("rig transcription provider wiring is disabled in this build; configure a provider (e.g. ZEOKEEL_TRANSCRIPTION_PROVIDER) after enabling the feature")
}
