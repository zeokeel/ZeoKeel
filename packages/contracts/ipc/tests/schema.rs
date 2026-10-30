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
        "ipc-command-request.json",
        &schemars::schema_for!(zk_contract_ipc::CommandRequest),
    );
    write(
        "ipc-command-response.json",
        &schemars::schema_for!(zk_contract_ipc::CommandResponse),
    );
    write(
        "ipc-event.json",
        &schemars::schema_for!(zk_contract_ipc::UiEvent),
    );
    write(
        "ipc-error.json",
        &schemars::schema_for!(zk_contract_ipc::IpcError),
    );
    write(
        "ipc-empty-payload.json",
        &schemars::schema_for!(zk_contract_ipc::payloads::EmptyPayload),
    );
    write(
        "ipc-workspace-create.json",
        &schemars::schema_for!(zk_contract_ipc::payloads::WorkspaceCreatePayload),
    );
    write(
        "ipc-workspace-id.json",
        &schemars::schema_for!(zk_contract_ipc::payloads::WorkspaceIdPayload),
    );
    write(
        "ipc-workspace-rename.json",
        &schemars::schema_for!(zk_contract_ipc::payloads::WorkspaceRenamePayload),
    );
    write(
        "ipc-workspace-remove.json",
        &schemars::schema_for!(zk_contract_ipc::payloads::WorkspaceRemovePayload),
    );
    write(
        "ipc-session-create.json",
        &schemars::schema_for!(zk_contract_ipc::payloads::SessionCreatePayload),
    );
    write(
        "ipc-session-messages.json",
        &schemars::schema_for!(zk_contract_ipc::payloads::SessionMessagesPayload),
    );
    write(
        "ipc-chat-send.json",
        &schemars::schema_for!(zk_contract_ipc::payloads::ChatSendPayload),
    );
    write(
        "ipc-chat-cancel.json",
        &schemars::schema_for!(zk_contract_ipc::payloads::ChatCancelPayload),
    );
    write(
        "ipc-settings-set.json",
        &schemars::schema_for!(zk_contract_ipc::payloads::SettingsSetPayload),
    );
    write(
        "ipc-approval-decide.json",
        &schemars::schema_for!(zk_contract_ipc::payloads::ApprovalDecidePayload),
    );
    write(
        "ipc-question-answer.json",
        &schemars::schema_for!(zk_contract_ipc::payloads::QuestionAnswerPayload),
    );
    write(
        "ipc-meta-info.json",
        &schemars::schema_for!(zk_contract_ipc::results::MetaInfo),
    );
    write(
        "ipc-chat-send-result.json",
        &schemars::schema_for!(zk_contract_ipc::results::ChatSendResult),
    );
    write(
        "workspace-change.json",
        &schemars::schema_for!(zk_contract_ipc::results::WorkspaceChange),
    );
    write(
        "workspace-changes-result.json",
        &schemars::schema_for!(zk_contract_ipc::results::WorkspaceChangesResult),
    );
}
