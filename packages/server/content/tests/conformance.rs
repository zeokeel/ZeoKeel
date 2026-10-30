#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only
#[test]
fn migration_is_immutable_and_present() {
    assert_eq!(zk_srv_content::MIGRATIONS.len(), 1);
    assert_eq!(zk_srv_content::MIGRATIONS[0].0, "0010_content");
}
