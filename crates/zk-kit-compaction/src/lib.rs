//! Conversation compaction summarizer (`/compact`, `2026-11-01-client-compact-command`).
//!
//! Pure planning/prompt/validation logic plus one provider-agnostic summarizer
//! call. `SessionStore::messages_in` keeps its raw semantics; the host builds the
//! compacted replay view and persists a `role=system` marker message.
//!
//! @capability kit.compaction
#![forbid(unsafe_code)]

use zk_contract_llm::LlmProvider;
use zk_contract_session::Message;

/// Recent complete turns kept verbatim after compaction (spec R2).
pub const KEEP_RECENT_TURNS: usize = 6;
/// Maximum recent raw messages kept verbatim after compaction.
pub const KEEP_RECENT_MESSAGES: usize = 12;
/// Maximum summarized input budget (tokens).
pub const MAX_INPUT_TOKENS: u64 = 12_000;
/// Maximum summary budget (tokens).
pub const MAX_SUMMARY_TOKENS: u64 = 2_000;

/// Closed compaction error set (host maps these to `compact/*` codes).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CompactionError {
    /// Input exceeds [`MAX_INPUT_TOKENS`].
    #[error("compact/too-large: input exceeds {0} tokens")]
    TooLarge(u64),
    /// Nothing older than the kept window exists.
    #[error("compact/noop: nothing to compact")]
    Noop,
    /// No usable provider (missing base_url/key or unsupported kind).
    #[error("compact/unavailable: {0}")]
    Unavailable(String),
    /// Provider call failed (network/timeout/status).
    #[error("compact/failed: {0}")]
    Provider(String),
    /// Summary exceeds [`MAX_SUMMARY_TOKENS`].
    #[error("compact/too-large: summary exceeds {0} tokens")]
    SummaryTooLarge(u64),
}

/// One compaction plan (bounds + prompt input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionRequest<'a> {
    /// Raw history prefix to summarize (oldest first; caller excludes markers).
    pub history: &'a [Message],
    /// Recent turns/messages kept verbatim (see [`plan_boundary`]).
    pub keep_recent: usize,
    /// Input budget (tokens).
    pub max_input_tokens: u64,
    /// Summary budget (tokens).
    pub max_summary_tokens: u64,
}

/// Successful summary result (nothing is persisted by this crate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionOutcome {
    /// Summary text (≤ summary budget).
    pub summary: String,
    /// Last raw message covered by the summary.
    pub through_message_id: String,
    /// Estimated input tokens.
    pub input_tokens: u64,
    /// Estimated summary tokens.
    pub output_tokens: u64,
    /// Number of raw messages folded into the summary.
    pub compressed_messages: u32,
}

/// Conservative token estimate (≈4 bytes per token, min 1 for non-empty text).
#[must_use]
pub fn estimate_tokens(text: &str) -> u64 {
    if text.is_empty() {
        return 0;
    }
    (u64::try_from(text.len()).unwrap_or(u64::MAX) / 4).saturating_add(1)
}

/// Whether `message` is a compaction marker (`role=system`, id prefix).
#[must_use]
pub fn is_marker(message: &Message) -> bool {
    message.role == "system" && message.id.starts_with("compaction:")
}

/// Index of the first globally-kept raw message, or `None` when nothing older exists.
///
/// Walks back from the newest message keeping at most `keep_recent_messages`
/// messages and at most `keep_recent_turns` user turns (whichever comes first).
#[must_use]
pub fn plan_boundary(
    messages: &[Message],
    keep_recent_turns: usize,
    keep_recent_messages: usize,
) -> Option<usize> {
    let raw: Vec<&Message> = messages
        .iter()
        .filter(|message| !is_marker(message))
        .collect();
    if raw.is_empty() || raw.len() <= 1 {
        return None;
    }
    let keep_turns = keep_recent_turns.clamp(1, KEEP_RECENT_TURNS);
    let keep_messages = keep_recent_messages.clamp(1, KEEP_RECENT_MESSAGES);
    let mut kept = 0_usize;
    let mut user_turns = 0_usize;
    let mut boundary = raw.len();
    for message in raw.iter().rev() {
        if kept >= keep_messages || user_turns >= keep_turns {
            break;
        }
        kept = kept.saturating_add(1);
        if message.role == "user" {
            user_turns = user_turns.saturating_add(1);
        }
        boundary = boundary.saturating_sub(1);
    }
    if boundary == 0 {
        None
    } else {
        Some(boundary)
    }
}

fn transcript(history: &[Message]) -> String {
    let mut out = String::new();
    for message in history {
        let role = if message.role == "tool" {
            "tool"
        } else {
            message.role.as_str()
        };
        out.push_str(&format!("[{role}] {}\n", message.content));
    }
    out
}

/// Build the summarization prompt (task objective/decisions/files/unfinished items).
#[must_use]
pub fn build_prompt(history: &[Message]) -> String {
    format!(
        "Summarize the earlier part of this conversation for continuation. Keep, without inventing:\n\
         - the user's goal and constraints\n\
         - decisions made and rejected options\n\
         - files/paths touched and relevant tool results\n\
         - unfinished work and next steps\n\
         Answer in the conversation's language. Plain text only, no preamble.\n\n\
         --- transcript ---\n{}",
        transcript(history)
    )
}

fn checked_input(history: &[Message], max_input_tokens: u64) -> Result<u64, CompactionError> {
    let tokens = estimate_tokens(&transcript(history));
    if tokens > max_input_tokens {
        return Err(CompactionError::TooLarge(tokens));
    }
    Ok(tokens)
}

fn checked_summary(summary: &str, max_summary_tokens: u64) -> Result<u64, CompactionError> {
    let tokens = estimate_tokens(summary);
    if tokens > max_summary_tokens {
        return Err(CompactionError::SummaryTooLarge(tokens));
    }
    Ok(tokens)
}

/// Summarize `request.history` with one provider call (no agent turn/tool rows).
///
/// # Errors
/// [`CompactionError`] for empty history, oversized input/summary, or provider failures;
/// no partial result is ever returned.
pub fn summarize_with_provider(
    provider: &dyn LlmProvider,
    request: &CompactionRequest<'_>,
) -> Result<CompactionOutcome, CompactionError> {
    let Some(last) = request.history.last() else {
        return Err(CompactionError::Noop);
    };
    let input_tokens = checked_input(request.history, request.max_input_tokens)?;
    let prompt = build_prompt(request.history);
    let summary = provider
        .complete(&prompt)
        .map_err(|error| CompactionError::Provider(error.to_string()))?;
    let summary = summary.trim().to_owned();
    if summary.is_empty() {
        return Err(CompactionError::Provider("empty summary".to_owned()));
    }
    let output_tokens = checked_summary(&summary, request.max_summary_tokens)?;
    Ok(CompactionOutcome {
        summary,
        through_message_id: last.id.clone(),
        input_tokens,
        output_tokens,
        compressed_messages: u32::try_from(request.history.len()).unwrap_or(u32::MAX),
    })
}

/// Deterministic extractive fallback (mock/offline provider; tagged in KNOWN-LIMITATIONS).
///
/// # Errors
/// Same bounded-input/summary errors as [`summarize_with_provider`].
pub fn summarize_extractive(
    request: &CompactionRequest<'_>,
) -> Result<CompactionOutcome, CompactionError> {
    let Some(last) = request.history.last() else {
        return Err(CompactionError::Noop);
    };
    let input_tokens = checked_input(request.history, request.max_input_tokens)?;
    let max_bytes =
        usize::try_from(request.max_summary_tokens.saturating_mul(4)).unwrap_or(usize::MAX);
    let mut summary = String::new();
    for message in request.history {
        if summary.len() >= max_bytes {
            break;
        }
        let line = format!("[{}] {}\n", message.role, message.content.trim());
        let remaining = max_bytes.saturating_sub(summary.len());
        let cut = line
            .char_indices()
            .take_while(|(index, _)| *index < remaining)
            .last()
            .map(|(index, character)| index.saturating_add(character.len_utf8()))
            .unwrap_or(0);
        summary.push_str(&line[..cut.min(line.len())]);
    }
    let summary = summary.trim().to_owned();
    if summary.is_empty() {
        return Err(CompactionError::Noop);
    }
    let output_tokens = checked_summary(&summary, request.max_summary_tokens)?;
    Ok(CompactionOutcome {
        summary,
        through_message_id: last.id.clone(),
        input_tokens,
        output_tokens,
        compressed_messages: u32::try_from(request.history.len()).unwrap_or(u32::MAX),
    })
}

/// Persisted marker content (JSON in `Message.content`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MarkerContent {
    /// Summary text.
    pub summary: String,
    /// Last raw message covered.
    pub through_message_id: String,
    /// Creation time (ms since epoch).
    pub created_ms: u64,
    /// Estimated input tokens.
    pub input_tokens: u64,
    /// Estimated summary tokens.
    pub output_tokens: u64,
    /// Number of compressed messages.
    pub compressed_messages: u32,
    /// Keep window used.
    pub keep_recent: u32,
}

impl MarkerContent {
    /// Parse marker JSON (corrupt markers are ignored by callers).
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        serde_json::from_str(raw).ok()
    }
}
