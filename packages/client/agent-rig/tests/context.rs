#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only

//! B10 subset: context budget from config (window override) + deterministic oldest-first
//! history truncation; refuse honestly when even the prompt alone does not fit.
use std::sync::Arc;

use rig_core::test_utils::{MockCompletionModel, MockTurn};
use zk_agent_rig::{AgentRuntimeConfig, ContextConfig, RigAgent};
use zk_contract_agent::{Agent, AgentEvent, EventSink, HistoryMessage, HistoryRole};
use zk_contract_tool::TurnContext;

struct NullSink;

impl EventSink for NullSink {
    fn emit(&self, _event: AgentEvent) {}
}

fn context() -> TurnContext {
    TurnContext {
        conversation_id: "c".to_owned(),
        workspace_id: "w".to_owned(),
        workspace_root: ".".to_owned(),
    }
}

fn agent(window_tokens: u64) -> (RigAgent, MockCompletionModel) {
    let model = MockCompletionModel::from_turns([MockTurn::text("done")]);
    let agent = RigAgent::new(model.clone().into(), Vec::new())
        .unwrap()
        .with_streaming(false)
        .with_agent_config(AgentRuntimeConfig {
            context: ContextConfig {
                window_tokens: Some(window_tokens),
            },
            ..AgentRuntimeConfig::default()
        });
    (agent, model)
}

fn long_history(count: usize) -> Vec<HistoryMessage> {
    (0..count)
        .map(|index| HistoryMessage {
            role: if index % 2 == 0 {
                HistoryRole::User
            } else {
                HistoryRole::Assistant
            },
            content: format!("history message number {index} with enough words to count as tokens"),
        })
        .collect()
}

#[test]
fn over_budget_history_is_trimmed_oldest_first() {
    // Window sized against the current preamble reserve (5_200 - 4_096 = 1_104 token
    // budget): the base prompt (~513 tokens) fits, 40 history messages still overflow.
    let (agent, model) = agent(5_200);
    let history = long_history(40);
    agent
        .run_stream_in_shared_with_history(&context(), "turn", "next", &history, Arc::new(NullSink))
        .expect("turn fits after trimming");
    let request = model.requests().first().cloned().expect("request");
    assert!(
        request.chat_history.len() < history.len(),
        "oldest history trimmed ({} -> {})",
        history.len(),
        request.chat_history.len()
    );
    let rendered = format!("{:?}", request.chat_history);
    assert!(
        !rendered.contains("number 0 "),
        "oldest message dropped first"
    );
    assert!(rendered.contains("number 39 "), "newest history kept");
}

#[test]
fn prompt_over_budget_is_refused_honestly() {
    let (agent, _model) = agent(1);
    let error = agent
        .run_stream_in_shared_with_history(&context(), "turn", "next", &[], Arc::new(NullSink))
        .expect_err("tiny window refuses the prompt");
    assert!(
        error.to_string().contains("CONTEXT_WINDOW_EXCEEDED"),
        "error names the budget failure: {error}"
    );
}
