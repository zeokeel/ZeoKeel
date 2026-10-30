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
        "agent-event.json",
        &schemars::schema_for!(zk_contract_agent::AgentEvent),
    );
    write(
        "agent-reply.json",
        &schemars::schema_for!(zk_contract_agent::AgentReply),
    );
    write(
        "agent-tool-call.json",
        &schemars::schema_for!(zk_contract_agent::ToolCallRecord),
    );
    write(
        "agent-approval-request.json",
        &schemars::schema_for!(zk_contract_agent::ApprovalRequest),
    );
    write(
        "agent-approval-decision.json",
        &schemars::schema_for!(zk_contract_agent::ApprovalDecision),
    );
    write(
        "agent-usage.json",
        &schemars::schema_for!(zk_contract_agent::AgentUsage),
    );
}
