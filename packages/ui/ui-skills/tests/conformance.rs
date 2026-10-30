#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
#[test]
fn manifest_exists() {
    assert!(std::path::Path::new("zk.toml").is_file());
}
