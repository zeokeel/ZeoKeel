//! Transcription capability placeholder.
#![forbid(unsafe_code)]

/// Provider availability; false until a concrete provider is configured.
#[must_use]
pub fn available() -> bool { false }

/// Closed unavailability code.
pub const UNAVAILABLE: &str = "CAPABILITY_UNAVAILABLE";
