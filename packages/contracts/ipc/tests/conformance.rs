//! Contract conformance: envelopes round-trip and errors are stable.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use serde_json::json;
use zk_contract_ipc::{CommandRequest, CommandResponse, UiEvent};

#[test]
fn envelopes_round_trip() {
    let req = CommandRequest { name: "chat.send".to_owned(), payload: json!({"text":"hi"}) };
    let text = serde_json::to_string(&req).unwrap();
    let back: CommandRequest = serde_json::from_str(&text).unwrap();
    assert_eq!(req, back);

    let ok = CommandResponse::ok(json!({"accepted":true}));
    assert!(ok.ok);
    assert!(ok.error.is_none());
    let err = CommandResponse::err("boom");
    assert!(!err.ok);

    let ev = UiEvent { stream_id: "s1".to_owned(), seq: 1, kind: "agent".to_owned(), payload: json!({"type":"text_delta","delta":"hi"}) };
    let text = serde_json::to_string(&ev).unwrap();
    let back: UiEvent = serde_json::from_str(&text).unwrap();
    assert_eq!(ev, back);
}
