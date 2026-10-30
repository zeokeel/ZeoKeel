#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
#[test]
fn unavailable_is_honest_and_explicit() {
    assert!(!zk_cap_image::available());
    assert!(zk_cap_image::unavailable_reason().is_some());
    assert_eq!(zk_cap_image::UNAVAILABLE, "CAPABILITY_UNAVAILABLE");
}
