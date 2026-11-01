#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only
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
    write(
        "approval-policy.json",
        &schemars::schema_for!(zk_contract_config::ApprovalPolicy),
    );
    write(
        "runtime-settings.json",
        &schemars::schema_for!(zk_contract_config::RuntimeSettings),
    );
    write(
        "schedule-settings.json",
        &schemars::schema_for!(zk_contract_config::ScheduleSettings),
    );
    write(
        "embedding-settings.json",
        &schemars::schema_for!(zk_contract_config::EmbeddingSettings),
    );
    write(
        "media-settings.json",
        &schemars::schema_for!(zk_contract_config::MediaSettings),
    );
    write(
        "web-search-settings.json",
        &schemars::schema_for!(zk_contract_config::WebSearchSettings),
    );

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join("shared/contracts/schema/embedding-settings.json");
    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let dims = &value["properties"]["dims"];
    assert_eq!(dims["minimum"], 1.0, "{dims}");
    assert_eq!(dims["maximum"], 65_536.0, "{dims}");

    // The generated schema must bound the runtime range (UI/Ajv consumers rely
    // on it); Rust validation stays authoritative.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let path = dir.join("shared/contracts/schema/schedule-settings.json");
    let text = std::fs::read_to_string(path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    let scan = &value["properties"]["scan_seconds"];
    assert_eq!(scan["minimum"], 5.0, "{scan}");
    assert_eq!(scan["maximum"], 3_600.0, "{scan}");

    // Web-search timeout bounds are published to the Ajv boundary too.
    let path = dir.join("shared/contracts/schema/web-search-settings.json");
    let text = std::fs::read_to_string(path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    let timeout = &value["properties"]["timeout_ms"];
    assert_eq!(timeout["minimum"], 1_000.0, "{timeout}");
    assert_eq!(timeout["maximum"], 120_000.0, "{timeout}");
}
