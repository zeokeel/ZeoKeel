#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
#[test]
fn manifest_exists_and_declares_personal_only_pane() {
    let manifest = std::fs::read_to_string("zk.toml").unwrap();
    assert!(manifest.contains("id = \"zeokeel/ui-settings-mcp\""));
    assert!(manifest.contains("component = \"settings.pane.mcp\""));
    assert!(manifest.contains("order = 70"));
    assert!(manifest.contains("editions = [\"personal\"]"));
}
