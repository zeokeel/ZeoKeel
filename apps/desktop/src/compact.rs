//! Conversation compaction (`/compact`, `2026-11-01-client-compact-command`).
//!
//! `SessionStore::messages_in` keeps raw semantics: this module builds the
//! read-only compacted replay view from raw messages + one persisted
//! `role=system` marker (`compaction:<uuid>`), and never deletes raw history.
#![forbid(unsafe_code)]

use std::sync::{Mutex, OnceLock};

use zk_composed::zk_kit_compaction::{
    self, CompactionError, CompactionRequest, MarkerContent, KEEP_RECENT_MESSAGES,
    KEEP_RECENT_TURNS, MAX_INPUT_TOKENS, MAX_SUMMARY_TOKENS,
};
use zk_contract_ipc::results::{
    CompactResult, CompactedViewResult, CompactionGetResult, CompactionMarker,
};
use zk_contract_session::{Message, SessionStore};

use crate::ipc::AppState;

/// Marker message id prefix (`compaction:<uuid>`).
pub const MARKER_PREFIX: &str = "compaction:";
/// Serializes compactions (one at a time; repeated requests are idempotent).
static COMPACT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn compact_lock() -> std::sync::MutexGuard<'static, ()> {
    COMPACT_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn storage(error: impl std::fmt::Display) -> String {
    format!("compact/failed: {error}")
}

/// Latest marker of one raw message list (id prefix + parseable content).
pub fn latest_marker(messages: &[Message]) -> Option<(Message, MarkerContent)> {
    messages.iter().rev().find_map(|message| {
        if message.role != "system" || !message.id.starts_with(MARKER_PREFIX) {
            return None;
        }
        MarkerContent::parse(&message.content).map(|parsed| (message.clone(), parsed))
    })
}

/// Raw messages with compaction markers removed (used for boundary planning).
fn raw_only(messages: &[Message]) -> Vec<Message> {
    messages
        .iter()
        .filter(|message| !zk_kit_compaction::is_marker(message))
        .cloned()
        .collect()
}

fn marker_dto(message: &Message, content: &MarkerContent) -> CompactionMarker {
    CompactionMarker {
        marker_id: message.id.clone(),
        summary: content.summary.clone(),
        through_message_id: content.through_message_id.clone(),
        created_ms: content.created_ms,
        input_tokens: content.input_tokens,
        output_tokens: content.output_tokens,
        compressed_messages: content.compressed_messages,
        keep_recent: content.keep_recent,
    }
}

/// The read-only compacted replay view: marker message + messages after its boundary.
///
/// Falls back to raw messages when there is no (valid) marker.
#[must_use]
pub fn compacted_messages(messages: &[Message]) -> Vec<Message> {
    let Some((marker_message, content)) = latest_marker(messages) else {
        return messages.to_vec();
    };
    let Some(boundary) = messages
        .iter()
        .position(|message| message.id == content.through_message_id)
    else {
        // Unknown boundary (corrupt/edited store): fall back to raw.
        return messages.to_vec();
    };
    let mut view = Vec::with_capacity(messages.len());
    view.push(marker_message);
    for message in messages.iter().skip(boundary.saturating_add(1)) {
        if zk_kit_compaction::is_marker(message) {
            continue;
        }
        view.push(message.clone());
    }
    view
}

/// Verify conversation ownership.
fn verify_owner(
    sessions: &dyn SessionStore,
    workspace_id: &str,
    conversation_id: &str,
) -> Result<(), String> {
    let conversations = sessions
        .list_conversations_in(workspace_id)
        .map_err(|error| error.to_string())?;
    if conversations
        .iter()
        .any(|conversation| conversation.id == conversation_id)
    {
        Ok(())
    } else {
        Err("session/workspace-mismatch".to_owned())
    }
}

/// Read the latest marker (`session.compaction_get`).
pub fn compaction_get(
    sessions: &dyn SessionStore,
    workspace_id: &str,
    conversation_id: &str,
) -> Result<CompactionGetResult, String> {
    verify_owner(sessions, workspace_id, conversation_id)?;
    let messages = sessions
        .messages_in(workspace_id, conversation_id)
        .map_err(|error| error.to_string())?;
    Ok(CompactionGetResult {
        marker: latest_marker(&messages).map(|(message, content)| marker_dto(&message, &content)),
    })
}

/// Read the compacted view (`session.compacted_view`); raw store is untouched.
pub fn compacted_view(
    sessions: &dyn SessionStore,
    workspace_id: &str,
    conversation_id: &str,
) -> Result<CompactedViewResult, String> {
    verify_owner(sessions, workspace_id, conversation_id)?;
    let messages = sessions
        .messages_in(workspace_id, conversation_id)
        .map_err(|error| error.to_string())?;
    Ok(CompactedViewResult {
        marker: latest_marker(&messages).map(|(message, content)| marker_dto(&message, &content)),
        messages: compacted_messages(&messages),
    })
}

fn compaction_error(error: CompactionError) -> String {
    match error {
        CompactionError::TooLarge(tokens) => {
            format!("compact/too-large: input needs {tokens} tokens (max {MAX_INPUT_TOKENS})")
        }
        CompactionError::SummaryTooLarge(tokens) => {
            format!("compact/too-large: summary needs {tokens} tokens (max {MAX_SUMMARY_TOKENS})")
        }
        CompactionError::Unavailable(reason) => format!("compact/unavailable: {reason}"),
        CompactionError::Provider(reason) => format!("compact/failed: {reason}"),
        CompactionError::Noop => "compact/noop".to_owned(),
    }
}

/// Run one compaction (`session.compact`).
pub fn compact_conversation(
    state: &std::sync::Arc<AppState>,
    workspace_id: &str,
    conversation_id: &str,
) -> Result<CompactResult, String> {
    let _guard = compact_lock();
    if state.is_conversation_running(conversation_id) {
        return Err("compact/busy: the conversation is streaming".to_owned());
    }
    let sessions = state.runtime().sessions();
    verify_owner(&*sessions, workspace_id, conversation_id)?;
    let messages = sessions
        .messages_in(workspace_id, conversation_id)
        .map_err(|error| error.to_string())?;
    let existing = latest_marker(&messages);
    let raw = raw_only(&messages);
    let Some(keep_boundary) =
        zk_kit_compaction::plan_boundary(&raw, KEEP_RECENT_TURNS, KEEP_RECENT_MESSAGES)
    else {
        return Ok(CompactResult {
            marker: existing.map(|(message, content)| marker_dto(&message, &content)),
            noop: true,
        });
    };
    let history: Vec<Message> = raw.iter().take(keep_boundary).cloned().collect();
    let Some(last_compacted) = history.last() else {
        return Ok(CompactResult {
            marker: existing.map(|(message, content)| marker_dto(&message, &content)),
            noop: true,
        });
    };
    if existing
        .as_ref()
        .is_some_and(|(_, content)| content.through_message_id == last_compacted.id)
    {
        // Same boundary as the latest marker: idempotent no-op.
        return Ok(CompactResult {
            marker: existing.map(|(message, content)| marker_dto(&message, &content)),
            noop: true,
        });
    }
    let request = CompactionRequest {
        history: &history,
        keep_recent: KEEP_RECENT_TURNS,
        max_input_tokens: MAX_INPUT_TOKENS,
        max_summary_tokens: MAX_SUMMARY_TOKENS,
    };
    let settings = state.runtime().settings().load().map_err(storage)?;
    let outcome = if settings.provider.kind == zk_contract_config::ProviderKind::Mock {
        // Offline/mock provider: deterministic extractive fallback (tagged in KNOWN).
        zk_kit_compaction::summarize_extractive(&request)
    } else {
        let prompt = zk_kit_compaction::build_prompt(&history);
        let tokens = zk_kit_compaction::estimate_tokens(
            &history
                .iter()
                .map(|message| message.content.clone())
                .collect::<Vec<String>>()
                .join("\n"),
        );
        if tokens > MAX_INPUT_TOKENS {
            return Err(compaction_error(CompactionError::TooLarge(tokens)));
        }
        let summary = state
            .runtime()
            .summarize(&prompt)
            .map_err(|error| format!("compact/failed: {error}"))?;
        let summary = summary.trim().to_owned();
        if summary.is_empty() {
            return Err("compact/failed: empty summary".to_owned());
        }
        let output_tokens = zk_kit_compaction::estimate_tokens(&summary);
        if output_tokens > MAX_SUMMARY_TOKENS {
            return Err(compaction_error(CompactionError::SummaryTooLarge(
                output_tokens,
            )));
        }
        Ok(zk_kit_compaction::CompactionOutcome {
            summary,
            through_message_id: last_compacted.id.clone(),
            input_tokens: tokens,
            output_tokens,
            compressed_messages: u32::try_from(history.len()).unwrap_or(u32::MAX),
        })
    }
    .map_err(compaction_error)?;
    let created_ms = now_ms();
    let content = MarkerContent {
        summary: outcome.summary,
        through_message_id: outcome.through_message_id,
        created_ms,
        input_tokens: outcome.input_tokens,
        output_tokens: outcome.output_tokens,
        compressed_messages: outcome.compressed_messages,
        keep_recent: u32::try_from(KEEP_RECENT_TURNS).unwrap_or(u32::MAX),
    };
    let marker_message = Message {
        id: format!("{MARKER_PREFIX}{}", uuid::Uuid::new_v4()),
        conversation_id: conversation_id.to_owned(),
        role: "system".to_owned(),
        content: serde_json::to_string(&content).map_err(storage)?,
        created_at_ms: created_ms,
        status: "done".to_owned(),
        usage: None,
        turn_id: String::new(),
    };
    // Any failure before this point leaves raw history untouched (no marker).
    sessions
        .append_message(&marker_message)
        .map_err(|error| format!("compact/failed: {error}"))?;
    Ok(CompactResult {
        marker: Some(marker_dto(&marker_message, &content)),
        noop: false,
    })
}
