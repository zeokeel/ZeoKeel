#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use zk_contract_llm::{LlmError, LlmProvider};
use zk_contract_session::Message;
use zk_kit_compaction::{
    self, CompactionError, CompactionRequest, MarkerContent, MAX_INPUT_TOKENS, MAX_SUMMARY_TOKENS,
};

struct FixedProvider(String);

impl LlmProvider for FixedProvider {
    fn complete(&self, _prompt: &str) -> Result<String, LlmError> {
        Ok(self.0.clone())
    }
}

struct FailingProvider;

impl LlmProvider for FailingProvider {
    fn complete(&self, _prompt: &str) -> Result<String, LlmError> {
        Err(LlmError::Upstream("boom".to_owned()))
    }
}

fn message(id: &str, role: &str, content: &str) -> Message {
    Message {
        id: id.to_owned(),
        conversation_id: "c1".to_owned(),
        role: role.to_owned(),
        content: content.to_owned(),
        created_at_ms: 1,
        status: "done".to_owned(),
        usage: None,
        turn_id: String::new(),
    }
}

fn history() -> Vec<Message> {
    vec![
        message("m1", "user", "goal: ship compaction"),
        message("m2", "assistant", "decision: keep raw store"),
        message("m3", "tool", "{\"path\":\"src/lib.rs\"}"),
        message("m4", "user", "next: wire the UI"),
        message("m5", "assistant", "kept recent answer"),
    ]
}

#[test]
fn estimates_tokens_and_plans_the_keep_window() {
    assert_eq!(zk_kit_compaction::estimate_tokens(""), 0);
    assert!(zk_kit_compaction::estimate_tokens("abcd") >= 1);
    let boundary = zk_kit_compaction::plan_boundary(&history(), 1, 2).expect("some older messages");
    assert!(boundary > 0 && boundary < 5);
    // Marker rows never count toward the window.
    let mut with_marker = history();
    with_marker.insert(
        0,
        message("compaction:abc", "system", "{\"summary\":\"old\"}"),
    );
    assert!(zk_kit_compaction::plan_boundary(&with_marker, 1, 1).is_some());
    // Everything fits in the keep window: no compactable prefix.
    assert!(zk_kit_compaction::plan_boundary(&history(), 99, 99).is_none());
}

#[test]
fn prompt_keeps_goal_decision_files_and_next_steps() {
    let prompt = zk_kit_compaction::build_prompt(&history());
    assert!(prompt.contains("goal and constraints"));
    assert!(prompt.contains("files/paths"));
    assert!(prompt.contains("ship compaction"));
    assert!(prompt.contains("src/lib.rs"));
}

#[test]
fn provider_summarize_is_bounded_and_has_no_partial_result() {
    let history = history();
    let request = CompactionRequest {
        history: &history,
        keep_recent: 1,
        max_input_tokens: MAX_INPUT_TOKENS,
        max_summary_tokens: MAX_SUMMARY_TOKENS,
    };
    let outcome = zk_kit_compaction::summarize_with_provider(
        &FixedProvider("short summary".to_owned()),
        &request,
    )
    .expect("summarize");
    assert_eq!(outcome.summary, "short summary");
    assert_eq!(outcome.through_message_id, "m5");
    assert_eq!(outcome.compressed_messages, 5);
    assert!(MarkerContent::parse(
        &serde_json::to_string(&MarkerContent {
            summary: outcome.summary.clone(),
            through_message_id: outcome.through_message_id.clone(),
            created_ms: 1,
            input_tokens: outcome.input_tokens,
            output_tokens: outcome.output_tokens,
            compressed_messages: outcome.compressed_messages,
            keep_recent: 1,
        })
        .unwrap()
    )
    .is_some());
    // Provider failures return an error (never a partial result).
    assert!(matches!(
        zk_kit_compaction::summarize_with_provider(&FailingProvider, &request),
        Err(CompactionError::Provider(_))
    ));
    // Oversized input is rejected before the provider is called.
    let oversized = CompactionRequest {
        history: &history,
        keep_recent: 1,
        max_input_tokens: 1,
        max_summary_tokens: MAX_SUMMARY_TOKENS,
    };
    assert!(matches!(
        zk_kit_compaction::summarize_with_provider(&FixedProvider("x".to_owned()), &oversized),
        Err(CompactionError::TooLarge(_))
    ));
    // Oversized summary is rejected.
    let big = "y".repeat(usize::try_from(MAX_SUMMARY_TOKENS).unwrap() * 4 + 64);
    assert!(matches!(
        zk_kit_compaction::summarize_with_provider(&FixedProvider(big), &request),
        Err(CompactionError::SummaryTooLarge(_))
    ));
}

#[test]
fn extractive_fallback_stays_within_budget() {
    let history = history();
    let request = CompactionRequest {
        history: &history,
        keep_recent: 1,
        max_input_tokens: MAX_INPUT_TOKENS,
        max_summary_tokens: MAX_SUMMARY_TOKENS,
    };
    let outcome = zk_kit_compaction::summarize_extractive(&request).expect("extractive");
    assert!(!outcome.summary.is_empty());
    assert!(outcome.output_tokens <= MAX_SUMMARY_TOKENS);
    assert_eq!(outcome.through_message_id, "m5");
    assert!(matches!(
        zk_kit_compaction::summarize_extractive(&CompactionRequest {
            history: &[],
            keep_recent: 1,
            max_input_tokens: MAX_INPUT_TOKENS,
            max_summary_tokens: MAX_SUMMARY_TOKENS,
        }),
        Err(CompactionError::Noop)
    ));
}
