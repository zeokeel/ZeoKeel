#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::path::PathBuf;

fn write(name: &str, schema: &schemars::schema::RootSchema) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = root.join("shared/contracts/schema");
    std::fs::create_dir_all(&dir).unwrap();
    let json = serde_json::to_string_pretty(schema).unwrap();
    std::fs::write(dir.join(name), format!("{json}\n")).unwrap();
}

#[test]
fn export_schema() {
    write(
        "app-settings.json",
        &schemars::schema_for!(zk_contract_config::AppSettings),
    );
    write(
        "provider-config.json",
        &schemars::schema_for!(zk_contract_config::ProviderConfig),
    );
    write(
        "provider-kind.json",
        &schemars::schema_for!(zk_contract_config::ProviderKind),
    );
    write(
        "agent-settings.json",
        &schemars::schema_for!(zk_contract_config::AgentSettings),
    );
    write(
        "tools-settings.json",
        &schemars::schema_for!(zk_contract_config::ToolSettings),
    );
}
