//! Process-wide current session store (for tools that read session history).
//!
//! @capability kit.session
#![forbid(unsafe_code)]
use std::sync::{Arc, Mutex, OnceLock};
use zk_contract_session::{SessionError, SessionSearchHit, SessionSearchResult, SessionStore};

static STORE: OnceLock<Mutex<Option<Arc<dyn SessionStore>>>> = OnceLock::new();
fn slot() -> &'static Mutex<Option<Arc<dyn SessionStore>>> {
    STORE.get_or_init(|| Mutex::new(None))
}

/// Install the desktop session store.
pub fn set_global(store: Arc<dyn SessionStore>) {
    if let Ok(mut guard) = slot().lock() {
        *guard = Some(store);
    }
}

/// Resolve the current session store.
#[must_use]
pub fn global() -> Option<Arc<dyn SessionStore>> {
    slot().lock().ok().and_then(|guard| guard.clone())
}

/// Maximum snippet context (characters) around a match (`2026-11-01-client-side-panels`).
const SNIPPET_RADIUS: usize = 60;

/// Full-text search inside one workspace (case-insensitive substring, snippet hits).
///
/// Single implementation for the user-facing `session.search` IPC and the
/// `session_search` tool (C17). Archived conversations are included (the UI
/// shows an `archived` marker); the tool keeps returning the full content so
/// agent behaviour is unchanged (P2-6 compatibility).
///
/// # Errors
/// Propagates [`SessionError`] from the store; an empty query yields an empty
/// result without touching the store.
pub fn search(
    store: &dyn SessionStore,
    workspace_id: &str,
    query: &str,
    limit: u32,
) -> Result<SessionSearchResult, SessionError> {
    let needle: Vec<char> = query.trim().to_lowercase().chars().collect();
    if needle.is_empty() {
        return Ok(SessionSearchResult {
            items: Vec::new(),
            truncated: false,
        });
    }
    let limit = usize::try_from(limit.clamp(1, 50)).unwrap_or(usize::MAX);
    let conversations = store.list_conversations_in(workspace_id)?;
    let mut items: Vec<SessionSearchHit> = Vec::new();
    let mut truncated = false;
    'conversations: for conversation in conversations {
        let messages = store.messages_in(workspace_id, &conversation.id)?;
        for message in messages {
            let haystack: Vec<char> = message.content.to_lowercase().chars().collect();
            if let Some(position) = find_chars(&haystack, &needle) {
                if items.len() >= limit {
                    truncated = true;
                    break 'conversations;
                }
                let content: Vec<char> = message.content.chars().collect();
                let start = position.saturating_sub(SNIPPET_RADIUS);
                let end = position
                    .saturating_add(needle.len())
                    .saturating_add(SNIPPET_RADIUS)
                    .min(content.len());
                let snippet: String = content
                    .get(start..end)
                    .map(|slice| slice.iter().collect())
                    .unwrap_or_default();
                items.push(SessionSearchHit {
                    conversation_id: conversation.id.clone(),
                    title: conversation.title.clone(),
                    message_id: message.id.clone(),
                    role: message.role.clone(),
                    snippet,
                    archived: conversation.archived,
                });
            }
        }
    }
    Ok(SessionSearchResult { items, truncated })
}

/// Case-insensitive character-substring search (returns the character index).
fn find_chars(haystack: &[char], needle: &[char]) -> Option<usize> {
    let first = *needle.first()?;
    haystack
        .iter()
        .enumerate()
        .filter(|(_, value)| **value == first)
        .map(|(index, _)| index)
        .find(|index| {
            needle.iter().enumerate().all(|(offset, expected)| {
                haystack
                    .get(index.saturating_add(offset))
                    .is_some_and(|value| value == expected)
            })
        })
}
