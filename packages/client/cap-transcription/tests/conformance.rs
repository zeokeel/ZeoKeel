#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test-only
//! Capability conformance: honest availability plus the closed error code.

use zk_cap_transcription::{TranscriptionError, CAPABILITY_UNAVAILABLE, UNAVAILABLE};

#[test]
fn unavailable_is_honest_and_explicit() {
    zk_cap_transcription::unconfigure().unwrap();
    assert!(!zk_cap_transcription::available());
    let reason = zk_cap_transcription::unavailable_reason().expect("reason");
    assert!(reason.contains("ZEOKEEL_TRANSCRIPTION_API_KEY"), "{reason}");
    assert_eq!(UNAVAILABLE, "CAPABILITY_UNAVAILABLE");
    assert_eq!(CAPABILITY_UNAVAILABLE, "CAPABILITY_UNAVAILABLE");
    match zk_cap_transcription::transcribe(b"audio-bytes", "clip.wav").unwrap_err() {
        TranscriptionError::Unavailable(message) => {
            assert!(
                message.contains("ZEOKEEL_TRANSCRIPTION_API_KEY"),
                "{message}"
            );
        }
        other => panic!("expected Unavailable, got {other:?}"),
    }
}
