//! Contract conformance: envelopes round-trip and errors are stable.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use serde_json::json;
use zk_contract_ipc::{CommandRequest, CommandResponse, UiEvent};

#[test]
fn envelopes_round_trip() {
    let req = CommandRequest {
        name: "chat.send".to_owned(),
        payload: json!({"text":"hi"}),
    };
    let text = serde_json::to_string(&req).unwrap();
    let back: CommandRequest = serde_json::from_str(&text).unwrap();
    assert_eq!(req, back);

    let ok = CommandResponse::ok(json!({"accepted":true}));
    assert!(ok.ok);
    assert!(ok.error.is_none());
    let err = CommandResponse::err("boom");
    assert!(!err.ok);

    let ev = UiEvent {
        stream_id: "s1".to_owned(),
        seq: 1,
        kind: "agent".to_owned(),
        payload: json!({"type":"text_delta","delta":"hi"}),
    };
    let text = serde_json::to_string(&ev).unwrap();
    let back: UiEvent = serde_json::from_str(&text).unwrap();
    assert_eq!(ev, back);
}

#[test]
fn workspace_changes_result_round_trips_and_warning_is_omitted_when_absent() {
    use zk_contract_ipc::results::{WorkspaceChange, WorkspaceChangesResult};

    let result = WorkspaceChangesResult {
        items: vec![WorkspaceChange {
            path: "src/main.rs".to_owned(),
            add: 1,
            del: 0,
        }],
        warning: None,
    };
    let text = serde_json::to_string(&result).unwrap();
    assert!(
        !text.contains("warning"),
        "clean result must omit warning: {text}"
    );
    let back: WorkspaceChangesResult = serde_json::from_str(&text).unwrap();
    assert_eq!(result, back);

    let warned: WorkspaceChangesResult =
        serde_json::from_str(r#"{"items":[],"warning":"not a git repository"}"#).unwrap();
    assert!(warned.items.is_empty());
    assert_eq!(warned.warning.as_deref(), Some("not a git repository"));
    let legacy: WorkspaceChangesResult = serde_json::from_str(r#"{"items":[]}"#).unwrap();
    assert!(legacy.warning.is_none());
}

#[test]
fn settings_revision_dtos_fence_compatibly() {
    use zk_contract_config::AppSettings;
    use zk_contract_ipc::payloads::SettingsSetPayload;
    use zk_contract_ipc::results::{SettingsGetResult, SettingsSetResult};

    let settings = AppSettings::default();

    // Legacy payload: `expected_revision` absent deserializes to `None` (no fence).
    let legacy: SettingsSetPayload =
        serde_json::from_value(serde_json::json!({ "settings": settings }))
            .expect("legacy settings.set payload must stay accepted");
    assert_eq!(legacy.expected_revision, None);
    // `None` is omitted on the wire (old hosts/clients keep the same shape).
    assert!(!serde_json::to_string(&legacy)
        .expect("serialize")
        .contains("expected_revision"));

    // New payload: the fence round-trips.
    let fenced = SettingsSetPayload {
        settings: settings.clone(),
        expected_revision: Some(7),
    };
    let text = serde_json::to_string(&fenced).expect("serialize fenced payload");
    assert!(text.contains("\"expected_revision\":7"));
    let back: SettingsSetPayload = serde_json::from_str(&text).expect("parse fenced payload");
    assert_eq!(back, fenced);

    // get/set results are structurally identical and keep the revision.
    let got = SettingsGetResult {
        settings: settings.clone(),
        revision: 3,
    };
    let put = SettingsSetResult {
        settings,
        revision: 4,
    };
    let got_value = serde_json::to_value(&got).expect("get result");
    let put_value = serde_json::to_value(&put).expect("set result");
    assert_eq!(got_value.get("revision"), Some(&serde_json::json!(3)));
    assert_eq!(put_value.get("revision"), Some(&serde_json::json!(4)));
}

#[test]
fn oauth_command_names_are_stable_and_unique() {
    use zk_contract_ipc::commands;

    // MCP client OAuth (docs/07 §6): authorize starts the loopback flow, status polls
    // (never returns tokens), refresh rotates a stored token in the background.
    assert_eq!(commands::OAUTH_AUTHORIZE, "oauth.authorize");
    assert_eq!(commands::OAUTH_STATUS, "oauth.status");
    assert_eq!(commands::OAUTH_REFRESH, "oauth.refresh");
    // The keychain token commands stay compatible with the existing UI.
    assert_eq!(commands::OAUTH_TOKEN_SAVE, "oauth.save");
    assert_eq!(commands::OAUTH_TOKEN_LOAD, "oauth.load");
    assert_eq!(commands::OAUTH_TOKEN_CLEAR, "oauth.clear");

    let names = [
        commands::CHAT_SEND,
        commands::CHAT_CANCEL,
        commands::SETTINGS_GET,
        commands::SETTINGS_SET,
        commands::OAUTH_TOKEN_SAVE,
        commands::OAUTH_TOKEN_LOAD,
        commands::OAUTH_TOKEN_CLEAR,
        commands::OAUTH_AUTHORIZE,
        commands::OAUTH_STATUS,
        commands::OAUTH_REFRESH,
        commands::SYSTEM_OPEN_URL,
        commands::WORKSPACE_ATTACHMENT_SAVE,
        commands::WORKSPACE_FILE_READ,
        commands::TERMINAL,
        commands::SESSION_UPDATE,
        commands::SESSION_FORK,
        commands::SESSION_SEARCH,
        commands::GOAL_GET,
        commands::GOAL_SET,
        commands::GOAL_UPDATE,
        commands::SESSION_OVERRIDE_GET,
        commands::SESSION_OVERRIDE_SET,
        commands::SESSION_EXPORT,
        commands::FEEDBACK_SUBMIT,
        commands::SESSION_COMPACT,
        commands::SESSION_COMPACTION_GET,
        commands::SESSION_COMPACTED_VIEW,
    ];
    let mut sorted = names.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "command names must be unique");
    assert!(names.iter().all(|name| !name.is_empty()));
}

#[test]
fn attachment_save_contract_round_trips() {
    use zk_contract_ipc::payloads::AttachmentSavePayload;
    use zk_contract_ipc::results::AttachmentSaveResult;

    assert_eq!(
        zk_contract_ipc::commands::WORKSPACE_ATTACHMENT_SAVE,
        "workspace.attachment_save"
    );

    let payload = AttachmentSavePayload {
        workspace_id: "ws-1".to_owned(),
        conversation_id: "conv-1".to_owned(),
        name: "shot.png".to_owned(),
        mime: "image/png".to_owned(),
        data_base64: "iVBORw0KGgo=".to_owned(),
    };
    let text = serde_json::to_string(&payload).unwrap();
    assert_eq!(
        payload,
        serde_json::from_str::<AttachmentSavePayload>(&text).unwrap()
    );

    let result = AttachmentSaveResult {
        path: ".zeokeel/attachments/conv-1/abc-shot.png".to_owned(),
        name: "shot.png".to_owned(),
        bytes: 8,
        mime: "image/png".to_owned(),
    };
    let value = serde_json::to_value(&result).unwrap();
    assert_eq!(
        value.get("path"),
        Some(&serde_json::json!(
            ".zeokeel/attachments/conv-1/abc-shot.png"
        ))
    );
}

#[test]
fn phase2b_payloads_round_trip() {
    use zk_contract_ipc::payloads::{
        SessionForkPayload, SessionSearchPayload, SessionUpdatePayload, TerminalAction,
        TerminalPayload, WorkspaceFileReadPayload, WorkspaceFilesPayload,
    };

    let files = WorkspaceFilesPayload {
        workspace_id: "ws-1".to_owned(),
        path: "demo-data".to_owned(),
        depth: Some(1),
        limit: Some(200),
    };
    assert_eq!(
        files,
        serde_json::from_str(&serde_json::to_string(&files).unwrap()).unwrap()
    );
    // Legacy no-argument payload parses with root/default knobs unset.
    let legacy: WorkspaceFilesPayload = serde_json::from_str(r#"{"workspace_id":"ws-1"}"#).unwrap();
    assert_eq!(legacy.path, "");
    assert_eq!(legacy.depth, None);
    assert_eq!(legacy.limit, None);

    let read = WorkspaceFileReadPayload {
        workspace_id: "ws-1".to_owned(),
        path: "demo-data/hello.txt".to_owned(),
        max_bytes: Some(1024),
    };
    assert_eq!(
        read,
        serde_json::from_str(&serde_json::to_string(&read).unwrap()).unwrap()
    );

    let terminal = TerminalPayload {
        workspace_id: "ws-1".to_owned(),
        conversation_id: "c-1".to_owned(),
        action: TerminalAction::Send,
        session: Some("default".to_owned()),
        command: Some("printf hi".to_owned()),
        timeout_ms: Some(5_000),
    };
    let text = serde_json::to_string(&terminal).unwrap();
    assert!(text.contains("\"action\":\"send\""));
    assert_eq!(
        terminal,
        serde_json::from_str::<TerminalPayload>(&text).unwrap()
    );

    let update = SessionUpdatePayload {
        workspace_id: "ws-1".to_owned(),
        conversation_id: "c-1".to_owned(),
        title: Some("renamed".to_owned()),
        pinned: Some(true),
        archived: Some(false),
    };
    assert_eq!(
        update,
        serde_json::from_str(&serde_json::to_string(&update).unwrap()).unwrap()
    );

    let fork = SessionForkPayload {
        workspace_id: "ws-1".to_owned(),
        conversation_id: "c-1".to_owned(),
        title: None,
    };
    assert_eq!(
        fork,
        serde_json::from_str(&serde_json::to_string(&fork).unwrap()).unwrap()
    );

    let search = SessionSearchPayload {
        workspace_id: "ws-1".to_owned(),
        query: "hello".to_owned(),
        limit: Some(20),
    };
    assert_eq!(
        search,
        serde_json::from_str(&serde_json::to_string(&search).unwrap()).unwrap()
    );
}

#[test]
fn command_surface_payloads_round_trip() {
    use zk_contract_ipc::payloads::{
        ConversationOverrideSetPayload, FeedbackSubmitPayload, GoalGetPayload, GoalSetPayload,
        SessionExportPayload,
    };
    let goal = GoalSetPayload {
        workspace_id: "w".to_owned(),
        conversation_id: "c".to_owned(),
        objective: "ship it".to_owned(),
        max_rounds: Some(3),
    };
    assert_eq!(
        goal,
        serde_json::from_str(&serde_json::to_string(&goal).unwrap()).unwrap()
    );
    let get = GoalGetPayload {
        workspace_id: "w".to_owned(),
        conversation_id: "c".to_owned(),
    };
    assert_eq!(
        get,
        serde_json::from_str(&serde_json::to_string(&get).unwrap()).unwrap()
    );
    let override_set = ConversationOverrideSetPayload {
        workspace_id: "w".to_owned(),
        conversation_id: "c".to_owned(),
        model: Some("deepseek-chat".to_owned()),
        reasoning_effort: Some("low".to_owned()),
        approval_policy: Some("strict".to_owned()),
    };
    assert_eq!(
        override_set,
        serde_json::from_str(&serde_json::to_string(&override_set).unwrap()).unwrap()
    );
    let export = SessionExportPayload {
        workspace_id: "w".to_owned(),
        conversation_id: "c".to_owned(),
        format: "markdown".to_owned(),
    };
    assert_eq!(
        export,
        serde_json::from_str(&serde_json::to_string(&export).unwrap()).unwrap()
    );
    let feedback = FeedbackSubmitPayload {
        workspace_id: "w".to_owned(),
        conversation_id: "c".to_owned(),
        category: "bug".to_owned(),
        description: Some("x".to_owned()),
    };
    assert_eq!(
        feedback,
        serde_json::from_str(&serde_json::to_string(&feedback).unwrap()).unwrap()
    );
}

#[test]
fn compaction_payloads_round_trip() {
    use zk_contract_ipc::payloads::{CompactionGetPayload, SessionCompactPayload};
    let compact = SessionCompactPayload {
        workspace_id: "w".to_owned(),
        conversation_id: "c".to_owned(),
    };
    assert_eq!(
        compact,
        serde_json::from_str(&serde_json::to_string(&compact).unwrap()).unwrap()
    );
    let get = CompactionGetPayload {
        workspace_id: "w".to_owned(),
        conversation_id: "c".to_owned(),
    };
    assert_eq!(
        get,
        serde_json::from_str(&serde_json::to_string(&get).unwrap()).unwrap()
    );
}
