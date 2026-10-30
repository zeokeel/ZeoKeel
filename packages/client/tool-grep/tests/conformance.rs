#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
#[test]
fn package_metadata() { assert!(std::path::Path::new("zk.toml").is_file()); }
