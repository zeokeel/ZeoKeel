//! Contract conformance: DTOs round-trip.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use zk_contract_agent::AgentUsage;
use zk_contract_session::{Conversation, Message, SessionStore};

#[test]
fn dto_round_trips() {
    let c = Conversation {
        id: "c1".to_owned(),
        workspace_id: "w1".to_owned(),
        title: "t".to_owned(),
        created_at_ms: 1,
        updated_at_ms: 2,
        pinned: false,
        archived: false,
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

#[test]
fn phase2b_conversation_defaults_are_backward_compatible() {
    let legacy = serde_json::json!({
        "id": "c1", "workspace_id": "w1", "title": "t",
        "created_at_ms": 1, "updated_at_ms": 2
    });
    let conversation: Conversation = serde_json::from_value(legacy).unwrap();
    assert!(!conversation.pinned);
    assert!(!conversation.archived);
}

#[test]
fn phase2b_trait_defaults_reject_without_breaking_implementations() {
    struct Bare;
    impl zk_contract_session::WorkspaceStore for Bare {
        fn list_workspaces(
            &self,
        ) -> Result<Vec<zk_contract_session::Workspace>, zk_contract_session::SessionError>
        {
            Ok(Vec::new())
        }
        fn create_workspace(
            &self,
            _path: &str,
            _title: &str,
        ) -> Result<zk_contract_session::Workspace, zk_contract_session::SessionError> {
            Err(zk_contract_session::SessionError::Invalid(
                "bare".to_owned(),
            ))
        }
        fn rename_workspace(
            &self,
            _workspace_id: &str,
            _title: &str,
        ) -> Result<zk_contract_session::Workspace, zk_contract_session::SessionError> {
            Err(zk_contract_session::SessionError::Invalid(
                "bare".to_owned(),
            ))
        }
        fn set_default_workspace(
            &self,
            _workspace_id: &str,
        ) -> Result<Vec<zk_contract_session::Workspace>, zk_contract_session::SessionError>
        {
            Ok(Vec::new())
        }
        fn remove_workspace(
            &self,
            _workspace_id: &str,
            _reassign_to: Option<&str>,
            _confirm: bool,
        ) -> Result<zk_contract_session::RemoveWorkspaceResult, zk_contract_session::SessionError>
        {
            Err(zk_contract_session::SessionError::Invalid(
                "bare".to_owned(),
            ))
        }
    }
    impl zk_contract_session::SessionStore for Bare {
        fn create_conversation_in(
            &self,
            _workspace_id: &str,
            _title: &str,
        ) -> Result<Conversation, zk_contract_session::SessionError> {
            Err(zk_contract_session::SessionError::Invalid(
                "bare".to_owned(),
            ))
        }
        fn list_conversations_in(
            &self,
            _workspace_id: &str,
        ) -> Result<Vec<Conversation>, zk_contract_session::SessionError> {
            Ok(Vec::new())
        }
        fn append_message(
            &self,
            _message: &zk_contract_session::Message,
        ) -> Result<(), zk_contract_session::SessionError> {
            Ok(())
        }
        fn messages_in(
            &self,
            _workspace_id: &str,
            _conversation_id: &str,
        ) -> Result<Vec<zk_contract_session::Message>, zk_contract_session::SessionError> {
            Ok(Vec::new())
        }
    }
    let bare = Bare;
    assert!(matches!(
        bare.rename_conversation("w", "c", "t"),
        Err(zk_contract_session::SessionError::Invalid(_))
    ));
    assert!(matches!(
        bare.set_pinned("w", "c", true),
        Err(zk_contract_session::SessionError::Invalid(_))
    ));
    assert!(matches!(
        bare.set_archived("w", "c", true),
        Err(zk_contract_session::SessionError::Invalid(_))
    ));
    assert!(matches!(
        bare.fork_conversation("w", "c", None),
        Err(zk_contract_session::SessionError::Invalid(_))
    ));
}
