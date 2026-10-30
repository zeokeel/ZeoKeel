#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::path::PathBuf;
#[test]
fn export_schema() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = root.join("shared/contracts/schema");
    std::fs::create_dir_all(&dir).unwrap();
    for (name, schema) in [
        ("app.json", schemars::schema_for!(zk_contract_app::AppItem)),
        ("app-frame.json", schemars::schema_for!(zk_contract_app::AppRequest)),
        ("app-manifest.json", schemars::schema_for!(zk_contract_app::AppManifest)),
    ] {
        std::fs::write(dir.join(name), format!("{}\n", serde_json::to_string_pretty(&schema).unwrap())).unwrap();
    }
}
