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
        "workspace.json",
        &schemars::schema_for!(zk_contract_session::Workspace),
    );
    write(
        "conversation.json",
        &schemars::schema_for!(zk_contract_session::Conversation),
    );
    write(
        "session-message.json",
        &schemars::schema_for!(zk_contract_session::Message),
    );
    write(
        "workspace-remove-result.json",
        &schemars::schema_for!(zk_contract_session::RemoveWorkspaceResult),
    );
    write(
        "file-entry.json",
        &schemars::schema_for!(zk_contract_session::FileEntry),
    );
    write(
        "workspace-files-result.json",
        &schemars::schema_for!(zk_contract_session::WorkspaceFilesResult),
    );
    write(
        "workspace-file-read-result.json",
        &schemars::schema_for!(zk_contract_session::WorkspaceFileReadResult),
    );
    write(
        "session-search-hit.json",
        &schemars::schema_for!(zk_contract_session::SessionSearchHit),
    );
    write(
        "session-search-result.json",
        &schemars::schema_for!(zk_contract_session::SessionSearchResult),
    );
}
