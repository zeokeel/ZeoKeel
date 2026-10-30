//! Conformance + behavior tests for `zk-llm-mock` (role `llm.provider.default`).
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use std::sync::Arc;

use zk_contract_llm::LlmProvider;
use zk_llm_mock::{provider, MockLlm};

/// Prompt exactly as the agent loop builds it.
fn agent_prompt(input: &str) -> String {
    format!("你是 ZeoKeel 最小 demo。\nUSER_INPUT:{input}\n如需读取文件只输出一行: TOOL fs.read <relative-path>")
}

#[test]
fn maps_chinese_read_request_to_tool_directive() {
    let llm = provider();
    assert_eq!(
        llm.complete(&agent_prompt("读取 demo-data/hello.txt"))
            .unwrap(),
        "TOOL fs.read demo-data/hello.txt"
    );
}

#[test]
fn echoes_plain_chinese_input() {
    let llm = provider();
    let reply = llm.complete(&agent_prompt("你好，今天天气不错")).unwrap();
    assert_eq!(reply, "（mock）收到：你好，今天天气不错");
}

#[test]
fn maps_english_read_request_and_trims_quotes() {
    let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm::new());
    assert_eq!(
        llm.complete("read \"demo-data/hello.txt\"").unwrap(),
        "TOOL fs.read demo-data/hello.txt"
    );
}

#[test]
fn trims_trailing_punctuation_from_token() {
    let llm = provider();
    assert_eq!(
        llm.complete(&agent_prompt("读取 demo-data/hello.txt。"))
            .unwrap(),
        "TOOL fs.read demo-data/hello.txt"
    );
}

#[test]
fn plain_prompt_without_marker_still_decides() {
    let llm = provider();
    assert_eq!(
        llm.complete("读取 demo-data/hello.txt").unwrap(),
        "TOOL fs.read demo-data/hello.txt"
    );
    assert_eq!(
        llm.complete("hello there").unwrap(),
        "（mock）收到：hello there"
    );
}

#[test]
fn read_requires_space_delimited_verb() {
    let llm = provider();
    assert_eq!(
        llm.complete("readme 只是文件名").unwrap(),
        "（mock）收到：readme 只是文件名"
    );
    assert_eq!(llm.complete("读取").unwrap(), "（mock）收到：读取");
}

#[test]
fn empty_prompt_does_not_panic() {
    let llm = provider();
    assert_eq!(llm.complete("").unwrap(), "（mock）收到：");
}
