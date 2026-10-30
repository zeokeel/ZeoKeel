#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::path::PathBuf;

#[test]
fn export_schema() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = root.join("shared/contracts/schema");
    std::fs::create_dir_all(&dir).unwrap();
    let schema = schemars::schema_for!(zk_contract_auth::LoginRequest);
    let json = serde_json::to_string_pretty(&schema).unwrap();
    std::fs::write(dir.join("auth-login.json"), format!("{json}\n")).unwrap();
}
