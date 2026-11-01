//! Contract conformance: DTOs round-trip.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use zk_contract_agent::AgentUsage;
use zk_contract_session::{Conversation, Message};

#[test]
fn dto_round_trips() {
    let c = Conversation {
        id: "c1".to_owned(),
        workspace_id: "w1".to_owned(),
        title: "t".to_owned(),
        created_at_ms: 1,
        updated_at_ms: 2,
    };
    let json = serde_json::to_string(&c).unwrap();
    assert_eq!(c, serde_json::from_str::<Conversation>(&json).unwrap());
    let m = Message {
        id: "m1".to_owned(),
        conversation_id: "c1".to_owned(),
        role: "user".to_owned(),
        content: "hi".to_owned(),
        created_at_ms: 3,
        status: "done".to_owned(),
        usage: Some(AgentUsage {
            input_tokens: 11,
            output_tokens: 5,
            total_tokens: 16,
            cached_tokens: 4,
            reasoning_tokens: 2,
            cost_micros: Some(42),
        }),
        turn_id: "turn-1".to_owned(),
    };
    let json = serde_json::to_string(&m).unwrap();
    assert_eq!(m, serde_json::from_str::<Message>(&json).unwrap());

    // Legacy message JSON without `usage` still parses as None.
    let legacy = serde_json::json!({
        "id": "m2", "conversation_id": "c1", "role": "assistant",
        "content": "old", "created_at_ms": 4, "status": "done"
    });
    let old: Message = serde_json::from_value(legacy).unwrap();
    assert!(old.usage.is_none());
}
