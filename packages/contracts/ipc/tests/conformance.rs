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
