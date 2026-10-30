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
        "tool-spec.json",
        &schemars::schema_for!(zk_contract_tool::ToolSpec),
    );
    write(
        "turn-context.json",
        &schemars::schema_for!(zk_contract_tool::TurnContext),
    );
    write(
        "tool-error.json",
        &schemars::schema_for!(zk_contract_tool::ToolError),
    );
}
