//! Ask-user question tool.
#![forbid(unsafe_code)]
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use zk_contract_agent::AgentEvent;
use zk_contract_tool::{Tool, ToolError, ToolSpec, TurnContext};

static SEQ: AtomicU64 = AtomicU64::new(1);

pub struct AskUserTool;
#[must_use]
pub fn tool() -> AskUserTool {
    AskUserTool
}
impl Tool for AskUserTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "ask_user_question".to_owned(),
            description:
                "Ask the user one or more questions before continuing; blocks until answered."
                    .to_owned(),
            parameters: serde_json::json!({"type":"object","properties":{"questions":{"type":"array","items":{"type":"object","properties":{"id":{"type":"string"},"question":{"type":"string"},"header":{"type":"string"},"options":{"type":"array","items":{"type":"object","properties":{"label":{"type":"string"},"description":{"type":"string"}},"required":["label"]}},"multi_select":{"type":"boolean"}},"required":["id","question"],"additionalProperties":false}}},"required":["questions"],"additionalProperties":false}),
            mutating: false,
            requires_approval: false,
        }
    }
    fn call(&self, _args: &str) -> Result<String, ToolError> {
        Err(ToolError::InvalidArgs(
            "ask_user_question requires JSON".to_owned(),
        ))
    }
    fn call_json(
        &self,
        _context: &TurnContext,
        args: serde_json::Value,
    ) -> Result<String, ToolError> {
        let questions = args
            .get("questions")
            .cloned()
            .ok_or_else(|| ToolError::InvalidArgs("questions".to_owned()))?;
        if !questions.as_array().is_some_and(|items| !items.is_empty()) {
            return Err(ToolError::InvalidArgs(
                "questions must be non-empty".to_owned(),
            ));
        }
        let broker = zk_kit_questions::global()
            .ok_or_else(|| ToolError::Denied("user question channel unavailable".to_owned()))?;
        let id = format!("question-{}", SEQ.fetch_add(1, Ordering::SeqCst));
        zk_kit_events::emit(AgentEvent::QuestionAsked {
            question_id: id.clone(),
            questions: questions.clone(),
        });
        let answers = broker.ask(&id, Duration::from_secs(300));
        zk_kit_events::emit(AgentEvent::QuestionAnswered {
            question_id: id,
            answers: answers.clone(),
        });
        if answers.is_null() {
            return Err(ToolError::Io("question timed out".to_owned()));
        }
        Ok(answers.to_string())
    }
}
