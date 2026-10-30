//! Deterministic mock LLM provider for the minimal demo.
//!
//! @capability llm.provider.default

#![forbid(unsafe_code)]

use zk_contract_llm::LlmProvider;

/// Marker used by the agent-loop prompt to delimit the user input.
const INPUT_MARKER: &str = "USER_INPUT:";

/// Prefix of the demo tool directive returned to the agent loop.
const TOOL_DIRECTIVE: &str = "TOOL ";

/// Chinese read verb marker (the trailing space is part of the trigger).
const READ_ZH: &str = "读取 ";

/// English read verb marker (the trailing space is part of the trigger).
const READ_EN: &str = "read ";

/// Deterministic provider: the decision comes from simple string matching.
#[derive(Debug, Clone, Copy, Default)]
pub struct MockLlm;

impl MockLlm {
    /// Create a mock provider.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

/// Default factory used by the generated glue (role `llm.provider.default`).
#[must_use]
pub fn provider() -> MockLlm {
    MockLlm::new()
}

impl LlmProvider for MockLlm {
    fn complete(&self, prompt: &str) -> Result<String, zk_contract_llm::LlmError> {
        let input = extract_user_input(prompt);
        if let Some(token) = read_token(&input) {
            return Ok(format!("{TOOL_DIRECTIVE}fs.read {token}"));
        }
        Ok(format!("（mock）收到：{input}"))
    }
}

/// Recover the user input from an agent-loop prompt or a raw input string.
fn extract_user_input(prompt: &str) -> String {
    let tail = match prompt.rfind(INPUT_MARKER) {
        Some(index) => {
            let start = index.saturating_add(INPUT_MARKER.len());
            prompt.get(start..).unwrap_or("")
        }
        None => prompt,
    };

    let mut last = String::new();
    for line in tail.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if is_instruction_line(trimmed) {
            if last.is_empty() {
                continue;
            }
            break;
        }
        last = trimmed.to_owned();
    }
    if last.is_empty() {
        tail.trim().to_owned()
    } else {
        last
    }
}

/// Prompt trailers after the user input are not part of the input.
fn is_instruction_line(line: &str) -> bool {
    line.starts_with("如需") || line.starts_with("如果") || line.starts_with("If you need")
}

/// Find `TOOL fs.read <token>` if the input asks to read a path.
fn read_token(input: &str) -> Option<&str> {
    find_marker_token(input, READ_ZH).or_else(|| find_marker_token(input, READ_EN))
}

fn find_marker_token<'a>(input: &'a str, marker: &str) -> Option<&'a str> {
    let mut search_from = 0usize;
    while let Some(found) = input.get(search_from..).and_then(|rest| rest.find(marker)) {
        let after = search_from
            .saturating_add(found)
            .saturating_add(marker.len());
        if let Some(rest) = input.get(after..) {
            if let Some(token) = first_token(rest) {
                return Some(token);
            }
        }
        search_from = after;
    }
    None
}

/// First whitespace-delimited token with surrounding quotes/punctuation trimmed.
fn first_token(rest: &str) -> Option<&str> {
    let raw = rest.split_whitespace().next()?;
    let trimmed = raw
        .trim_start_matches(is_wrap_or_punct)
        .trim_end_matches(is_wrap_or_punct);
    (!trimmed.is_empty()).then_some(trimmed)
}

/// Punctuation that wraps a path in prose; path characters (`/`, `.`, `-`) are kept.
fn is_wrap_or_punct(character: char) -> bool {
    matches!(
        character,
        '"' | '\''
            | '`'
            | '“'
            | '”'
            | '‘'
            | '’'
            | '《'
            | '》'
            | '「'
            | '」'
            | '（'
            | '）'
            | '('
            | ')'
            | '['
            | ']'
            | '<'
            | '>'
            | '，'
            | '。'
            | '、'
            | '；'
            | '：'
            | '！'
            | '？'
            | ','
            | ';'
            | ':'
            | '!'
            | '?'
    )
}
