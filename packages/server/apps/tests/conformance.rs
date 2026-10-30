#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only
#[test]
fn migration_id() {
    assert_eq!(zk_srv_apps::MIGRATIONS[0].0, "0012_apps");
}
