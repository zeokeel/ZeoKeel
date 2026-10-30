#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
#[test]
fn contribution() {
    assert_eq!(zk_ui_login::contributions().len(), 1);
}
