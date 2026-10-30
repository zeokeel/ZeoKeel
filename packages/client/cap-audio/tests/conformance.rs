#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test-only
//! Capability conformance: honest availability plus the closed error code.

use zk_cap_audio::{AudioError, CAPABILITY_UNAVAILABLE, UNAVAILABLE};

#[test]
fn unavailable_is_honest_and_explicit() {
    zk_cap_audio::unconfigure().unwrap();
    assert!(!zk_cap_audio::available());
    let reason = zk_cap_audio::unavailable_reason().expect("reason");
    assert!(reason.contains("ZEOKEEL_AUDIO_API_KEY"), "{reason}");
    assert_eq!(UNAVAILABLE, "CAPABILITY_UNAVAILABLE");
    assert_eq!(CAPABILITY_UNAVAILABLE, "CAPABILITY_UNAVAILABLE");
    match zk_cap_audio::synthesize("hello", "alloy").unwrap_err() {
        AudioError::Unavailable(message) => {
            assert!(message.contains("ZEOKEEL_AUDIO_API_KEY"), "{message}");
        }
        other => panic!("expected Unavailable, got {other:?}"),
    }
}
