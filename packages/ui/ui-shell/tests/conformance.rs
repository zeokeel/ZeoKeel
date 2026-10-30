#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

#[test]
fn contributions_are_declared() {
    assert!(!zk_ui_shell::contributions().is_empty());
}
