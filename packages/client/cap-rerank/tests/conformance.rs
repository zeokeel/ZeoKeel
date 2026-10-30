#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
#[test]
fn unavailable_is_honest_and_explicit() {
    assert!(!zk_cap_rerank::available());
    assert!(zk_cap_rerank::unavailable_reason().is_some());
    assert_eq!(zk_cap_rerank::UNAVAILABLE, "CAPABILITY_UNAVAILABLE");
}
