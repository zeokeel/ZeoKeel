#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
#[test]
fn unavailable_is_explicit() {
    assert!(!zk_cap_image::available());
}
