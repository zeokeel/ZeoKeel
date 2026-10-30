#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
#[test]
fn unavailable_is_honest_and_explicit() {
    assert!(!zk_cap_transcription::available());
    assert!(zk_cap_transcription::unavailable_reason().is_some());
    assert_eq!(zk_cap_transcription::UNAVAILABLE, "CAPABILITY_UNAVAILABLE");
}
