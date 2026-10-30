#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
#[test]
fn export_schema() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = root.join("shared/contracts/schema");
    std::fs::create_dir_all(&dir).unwrap();
    let schema = schemars::schema_for!(zk_contract_content::Content);
    std::fs::write(dir.join("content.json"), format!("{}\n", serde_json::to_string_pretty(&schema).unwrap())).unwrap();
}
