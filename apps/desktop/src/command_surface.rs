//! DSH 1:1 command surface (Batch 1): conversation-scoped overrides, goals,
//! Markdown export and local feedback (`2026-11-01-client-command-surface`).
#![forbid(unsafe_code)]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use serde_json::Value;
use zk_contract_config::AppSettings;
use zk_contract_ipc::payloads::{GoalAction, GoalSetPayload, GoalUpdatePayload};
use zk_contract_ipc::results::{
    ConversationOverride, ConversationOverrideResult, FeedbackSubmitResult, GoalResult, GoalView,
    SessionExportResult,
};
use zk_contract_session::{Message, SessionStore};
use zk_contract_tool::TurnContext;

/// Maximum exported Markdown size (bytes).
const MAX_EXPORT_BYTES: usize = 8 * 1024 * 1024;
/// Maximum feedback description size (bytes).
const MAX_FEEDBACK_BYTES: usize = 4096;
/// Maximum model id length (chars).
const MAX_MODEL_CHARS: usize = 200;
/// Approval policy presets accepted from the UI (DSH mapping, approval only).
pub const APPROVAL_PRESETS: &[&str] = &["strict", "auto_read_only", "ask"];
/// Reasoning-effort levels accepted from the UI.
pub const REASONING_EFFORTS: &[&str] = &["minimal", "low", "medium", "high", "xhigh", "max"];

/// Process-wide config mutex: one conversation may mutate the global agent
/// config (model/agent settings/approval policy) at a time.
static CONFIG_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Acquire the global config mutex (poisoning is recovered, never panics).
pub fn lock_config() -> MutexGuard<'static, ()> {
    let mutex = CONFIG_MUTEX.get_or_init(|| Mutex::new(()));
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn storage(error: impl std::fmt::Display) -> String {
    format!("command-surface storage: {error}")
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// Resolve one workspace root (canonical absolute path) for `workspace_id`.
fn workspace_root(sessions: &dyn SessionStore, workspace_id: &str) -> Result<PathBuf, String> {
    let workspaces = sessions
        .list_workspaces()
        .map_err(|error| error.to_string())?;
    let Some(workspace) = workspaces
        .into_iter()
        .find(|workspace| workspace.id == workspace_id)
    else {
        return Err("workspace/not-found".to_owned());
    };
    std::fs::canonicalize(&workspace.path)
        .map_err(|error| format!("workspace/path-unavailable: {error}"))
}

/// Verify the conversation belongs to the workspace and return its message list.
fn conversation_messages(
    sessions: &dyn SessionStore,
    workspace_id: &str,
    conversation_id: &str,
) -> Result<(String, Vec<Message>), String> {
    let conversations = sessions
        .list_conversations_in(workspace_id)
        .map_err(|error| error.to_string())?;
    let Some(conversation) = conversations
        .into_iter()
        .find(|conversation| conversation.id == conversation_id)
    else {
        return Err("session/workspace-mismatch".to_owned());
    };
    let messages = sessions
        .messages_in(workspace_id, conversation_id)
        .map_err(|error| error.to_string())?;
    Ok((conversation.title, messages))
}

/// One conversation's persisted override record.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct OverrideRecord {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    reasoning_effort: Option<String>,
    #[serde(default)]
    approval_policy: Option<String>,
    #[serde(default)]
    updated_ms: u64,
}

impl OverrideRecord {
    fn is_empty(&self) -> bool {
        self.model.is_none() && self.reasoning_effort.is_none() && self.approval_policy.is_none()
    }
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct OverrideFile {
    #[serde(default)]
    schema: u32,
    #[serde(default)]
    overrides: std::collections::BTreeMap<String, OverrideRecord>,
}

fn overrides_path(root: &Path) -> Result<PathBuf, String> {
    zk_composed::zk_kit_tools::safe_join_under(root, &[".zeokeel", "conversation-overrides.json"])
        .map_err(|error| format!("workspace/path-unavailable: {error}"))
}

fn load_overrides(root: &Path) -> OverrideFile {
    let Ok(path) = overrides_path(root) else {
        return OverrideFile::default();
    };
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return OverrideFile::default();
    };
    match serde_json::from_str::<OverrideFile>(&raw) {
        Ok(mut file) => {
            // Corrupt/legacy documents degrade to defaults instead of blocking turns.
            if file.schema != 1 {
                file.schema = 1;
            }
            file
        }
        Err(error) => {
            eprintln!("command-surface: ignoring corrupt conversation overrides: {error}");
            OverrideFile::default()
        }
    }
}

fn save_overrides(root: &Path, file: &OverrideFile) -> Result<(), String> {
    let path = overrides_path(root)?;
    let mut payload = serde_json::to_vec_pretty(file).map_err(storage)?;
    payload.push(b'\n');
    let tmp = path.with_extension("json.tmp");
    {
        let mut handle = std::fs::File::create(&tmp).map_err(storage)?;
        handle.write_all(&payload).map_err(storage)?;
        handle.flush().map_err(storage)?;
    }
    std::fs::rename(&tmp, &path).map_err(storage)?;
    Ok(())
}

fn record_to_view(record: &OverrideRecord) -> ConversationOverride {
    ConversationOverride {
        model: record.model.clone(),
        reasoning_effort: record.reasoning_effort.clone(),
        approval_policy: record.approval_policy.clone(),
        is_session_override: !record.is_empty(),
    }
}

fn checked_model(model: &str, enterprise_models: &[String]) -> Result<String, String> {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return Err("session/invalid: empty model".to_owned());
    }
    if trimmed.chars().count() > MAX_MODEL_CHARS {
        return Err("session/invalid: model too long".to_owned());
    }
    if trimmed.chars().any(char::is_control) {
        return Err("session/invalid: control character in model".to_owned());
    }
    if !enterprise_models.is_empty() && !enterprise_models.iter().any(|known| known == trimmed) {
        return Err("session/invalid: model is not in the gateway model list".to_owned());
    }
    Ok(trimmed.to_owned())
}

fn checked_effort(effort: &str) -> Result<String, String> {
    let trimmed = effort.trim();
    if REASONING_EFFORTS.contains(&trimmed) {
        Ok(trimmed.to_owned())
    } else {
        Err("session/invalid: unknown reasoning effort".to_owned())
    }
}

fn checked_policy(policy: &str) -> Result<String, String> {
    let trimmed = policy.trim();
    if APPROVAL_PRESETS.contains(&trimmed) {
        Ok(trimmed.to_owned())
    } else {
        Err("session/invalid: unknown approval policy".to_owned())
    }
}

/// Read the persisted conversation override (no application side effects).
pub fn get_override(
    sessions: &dyn SessionStore,
    workspace_id: &str,
    conversation_id: &str,
) -> Result<ConversationOverrideResult, String> {
    let root = workspace_root(sessions, workspace_id)?;
    // Ownership is verified even when no override exists.
    let _ = conversation_messages(sessions, workspace_id, conversation_id)?;
    let file = load_overrides(&root);
    Ok(ConversationOverrideResult {
        overrides: file
            .overrides
            .get(conversation_id)
            .map_or_else(default_marker, record_to_view),
    })
}

fn default_marker() -> ConversationOverride {
    ConversationOverride {
        model: None,
        reasoning_effort: None,
        approval_policy: None,
        is_session_override: false,
    }
}

/// Write the conversation override (`None` clears one field; all-empty removes it).
///
/// `enterprise_models` is the host-cached gateway model list (empty for personal).
pub fn set_override(
    sessions: &dyn SessionStore,
    workspace_id: &str,
    conversation_id: &str,
    model: Option<&str>,
    reasoning_effort: Option<&str>,
    approval_policy: Option<&str>,
    enterprise_models: &[String],
) -> Result<ConversationOverrideResult, String> {
    let root = workspace_root(sessions, workspace_id)?;
    let _ = conversation_messages(sessions, workspace_id, conversation_id)?;
    let mut file = load_overrides(&root);
    let mut record = file
        .overrides
        .get(conversation_id)
        .cloned()
        .unwrap_or_default();
    if let Some(model) = model {
        record.model = if model.trim().is_empty() {
            None // explicit clear
        } else {
            Some(checked_model(model, enterprise_models)?)
        };
    }
    if let Some(effort) = reasoning_effort {
        record.reasoning_effort = if effort.trim().is_empty() {
            None
        } else {
            Some(checked_effort(effort)?)
        };
    }
    if let Some(policy) = approval_policy {
        record.approval_policy = if policy.trim().is_empty() {
            None
        } else {
            Some(checked_policy(policy)?)
        };
    }
    record.updated_ms = now_ms();
    if record.is_empty() {
        file.overrides.remove(conversation_id);
    } else {
        file.overrides.insert(conversation_id.to_owned(), record);
    }
    file.schema = 1;
    save_overrides(&root, &file)?;
    Ok(ConversationOverrideResult {
        overrides: file
            .overrides
            .get(conversation_id)
            .map_or_else(default_marker, record_to_view),
    })
}

/// Snapshot the global settings for the config-mutex restore path.
pub fn snapshot_settings(
    settings: &dyn zk_contract_config::SettingsStore,
) -> Result<AppSettings, String> {
    settings
        .load()
        .map_err(|error| format!("settings/load: {error}"))
}

fn provider_kind(settings: &AppSettings) -> &'static str {
    match settings.provider.kind {
        zk_contract_config::ProviderKind::Mock => "mock",
        zk_contract_config::ProviderKind::Ollama => "ollama",
        zk_contract_config::ProviderKind::OpenAiCompatible => "open_ai_compatible",
    }
}

/// Apply one conversation's override to the shared agent (caller holds the mutex).
///
/// Returns whether an override was applied (restore is still required to clear
/// the approval preset even when nothing was applied, so callers always restore).
pub fn apply_override(
    runtime: &zk_composed::Runtime,
    global: &AppSettings,
    workspace_id: &str,
    conversation_id: &str,
) -> Result<bool, String> {
    let root = workspace_root(&*runtime.sessions(), workspace_id)?;
    let file = load_overrides(&root);
    let Some(record) = file.overrides.get(conversation_id) else {
        return Ok(false);
    };
    let mut settings = global.clone();
    if let Some(model) = record.model.as_deref() {
        if matches!(
            settings.provider.kind,
            zk_contract_config::ProviderKind::Mock
        ) {
            // The mock provider ignores model ids; no hot-swap is required.
        } else if runtime.packages().contains(&"zeokeel/llm-gateway") {
            let (server_url, token) = zk_composed::gateway_credentials()
                .ok_or_else(|| "session/invalid: gateway credentials unavailable".to_owned())?;
            runtime
                .configure_llm(&server_url, &token, model, "gateway")
                .map_err(|error| format!("session/invalid: {error}"))?;
        } else {
            runtime
                .configure_llm(
                    &settings.provider.base_url,
                    &settings.provider.api_key,
                    model,
                    provider_kind(&settings),
                )
                .map_err(|error| format!("session/invalid: {error}"))?;
        }
        settings.provider.model = model.to_owned();
    }
    if let Some(effort) = record.reasoning_effort.as_deref() {
        settings.agent.completion.reasoning_effort = Some(effort.to_owned());
    }
    settings
        .agent
        .validate()
        .map_err(|error| format!("session/invalid: {error}"))?;
    runtime
        .configure_agent_runtime(&settings.agent)
        .map_err(|error| format!("session/invalid: {error}"))?;
    if let Some(policy) = record.approval_policy.as_deref() {
        runtime
            .set_approval_preset(Some(policy))
            .map_err(|error| format!("session/invalid: {error}"))?;
    }
    Ok(true)
}

/// Restore the global settings (caller holds the mutex; no-op without overrides).
pub fn restore_global(runtime: &zk_composed::Runtime, global: &AppSettings) -> Result<(), String> {
    if !matches!(global.provider.kind, zk_contract_config::ProviderKind::Mock) {
        runtime
            .configure_llm(
                &global.provider.base_url,
                &global.provider.api_key,
                &global.provider.model,
                provider_kind(global),
            )
            .map_err(|error| format!("settings/apply: {error}"))?;
    }
    runtime
        .configure_agent_runtime(&global.agent)
        .map_err(|error| format!("settings/apply: {error}"))?;
    runtime
        .set_approval_preset(Some(global.tools.effective_approval_policy().as_str()))
        .map_err(|error| format!("settings/apply: {error}"))?;
    Ok(())
}

fn goal_context(
    sessions: &dyn SessionStore,
    workspace_id: &str,
    conversation_id: &str,
) -> Result<TurnContext, String> {
    let _ = conversation_messages(sessions, workspace_id, conversation_id)?;
    let root = workspace_root(sessions, workspace_id)?;
    Ok(TurnContext {
        conversation_id: conversation_id.to_owned(),
        workspace_id: workspace_id.to_owned(),
        workspace_root: root.to_string_lossy().into_owned(),
    })
}

fn goal_view(value: &Value) -> Result<GoalView, String> {
    serde_json::from_value::<GoalView>(value.clone())
        .map_err(|error| format!("goal/invalid: {error}"))
}

/// Read the conversation goal (same `.zeokeel/goal.json` as `tool.goal`).
pub fn goal_get(
    sessions: &dyn SessionStore,
    workspace_id: &str,
    conversation_id: &str,
) -> Result<GoalResult, String> {
    let context = goal_context(sessions, workspace_id, conversation_id)?;
    let goal = zk_composed::zk_tool_goal::driver::goal(&context)
        .map_err(|error| format!("goal/error: {error}"))?;
    match goal {
        Some(value) => Ok(GoalResult {
            goal: Some(goal_view(&value)?),
        }),
        None => Ok(GoalResult { goal: None }),
    }
}

/// Set/replace the conversation goal (inline confirmation is a UI concern).
pub fn goal_set(
    sessions: &dyn SessionStore,
    request: &GoalSetPayload,
) -> Result<GoalResult, String> {
    let context = goal_context(sessions, &request.workspace_id, &request.conversation_id)?;
    let value = zk_composed::zk_tool_goal::driver::set_goal(
        &context,
        &request.objective,
        request.max_rounds,
    )
    .map_err(|error| format!("session/invalid: {error}"))?;
    Ok(GoalResult {
        goal: Some(goal_view(&value)?),
    })
}

/// Pause/resume/cancel the conversation goal.
pub fn goal_update(
    sessions: &dyn SessionStore,
    request: &GoalUpdatePayload,
) -> Result<GoalResult, String> {
    let context = goal_context(sessions, &request.workspace_id, &request.conversation_id)?;
    let value = match request.action {
        GoalAction::Pause => zk_composed::zk_tool_goal::driver::update_goal(
            &context,
            Some("blocked"),
            Some("paused by user"),
            None,
        ),
        GoalAction::Resume => zk_composed::zk_tool_goal::driver::update_goal(
            &context,
            Some("active"),
            Some("resumed by user"),
            None,
        ),
        GoalAction::Cancel => zk_composed::zk_tool_goal::driver::cancel_goal(&context),
    }
    .map_err(|error| format!("session/invalid: {error}"))?;
    Ok(GoalResult {
        goal: Some(goal_view(&value)?),
    })
}

fn sanitize_export_name(title: &str) -> String {
    let mut name = String::new();
    for character in title.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
            name.push(character);
        } else if character.is_whitespace() {
            name.push('-');
        }
        if name.len() >= 60 {
            break;
        }
    }
    let name = name.trim_matches(['-', '.']).to_owned();
    if name.is_empty() {
        "conversation".to_owned()
    } else {
        name
    }
}

fn export_markdown(title: &str, messages: &[Message]) -> String {
    let mut out = format!("# {title}\n\n");
    for message in messages {
        out.push_str(&format!(
            "## {} ({})\n\n{}\n\n",
            message.role, message.created_at_ms, message.content
        ));
        if let Some(usage) = message.usage.as_ref() {
            out.push_str(&format!(
                "> usage: in {} / out {} / total {}\n\n",
                usage.input_tokens, usage.output_tokens, usage.total_tokens
            ));
        }
    }
    out
}

/// Export one conversation as Markdown under `<workspace>/.zeokeel/exports/`.
pub fn export_markdown_file(
    sessions: &dyn SessionStore,
    workspace_id: &str,
    conversation_id: &str,
) -> Result<SessionExportResult, String> {
    let (title, messages) = conversation_messages(sessions, workspace_id, conversation_id)?;
    if messages.is_empty() {
        return Err("session/invalid: conversation has no messages to export".to_owned());
    }
    let root = workspace_root(sessions, workspace_id)?;
    let mut body = export_markdown(&title, &messages);
    let mut truncated = false;
    if body.len() > MAX_EXPORT_BYTES {
        let mut boundary = MAX_EXPORT_BYTES;
        while boundary > 0 && !body.is_char_boundary(boundary) {
            boundary = boundary.saturating_sub(1);
        }
        body.truncate(boundary);
        truncated = true;
    }
    let filename = format!("{}-{}.md", sanitize_export_name(&title), now_ms());
    let path =
        zk_composed::zk_kit_tools::safe_join_under(&root, &[".zeokeel", "exports", &filename])
            .map_err(|error| format!("workspace/path-escape: {error}"))?;
    let bytes = u64::try_from(body.len()).unwrap_or(u64::MAX);
    std::fs::write(&path, body).map_err(storage)?;
    Ok(SessionExportResult {
        path: path.to_string_lossy().into_owned(),
        bytes,
        mime: "text/markdown".to_owned(),
        truncated,
    })
}

/// Store local-only feedback JSON under `<workspace>/.zeokeel/feedback/`.
pub fn submit_feedback(
    sessions: &dyn SessionStore,
    workspace_id: &str,
    conversation_id: &str,
    category: &str,
    description: Option<&str>,
    edition: &str,
) -> Result<FeedbackSubmitResult, String> {
    if !matches!(category, "bug" | "idea" | "other") {
        return Err("session/invalid: unknown feedback category".to_owned());
    }
    let description = description.unwrap_or("").trim();
    if description.len() > MAX_FEEDBACK_BYTES {
        return Err("session/invalid: feedback description exceeds 4 KiB".to_owned());
    }
    let _ = conversation_messages(sessions, workspace_id, conversation_id)?;
    let root = workspace_root(sessions, workspace_id)?;
    let filename = format!("{}-{}.json", now_ms(), uuid::Uuid::new_v4());
    let path =
        zk_composed::zk_kit_tools::safe_join_under(&root, &[".zeokeel", "feedback", &filename])
            .map_err(|error| format!("workspace/path-escape: {error}"))?;
    let payload = serde_json::json!({
        "schema": 1,
        "category": category,
        "description": description,
        "edition": edition,
        "version": env!("CARGO_PKG_VERSION"),
        "platform": std::env::consts::OS,
        "workspace_id": workspace_id,
        "conversation_id": conversation_id,
        "at_ms": now_ms(),
    });
    std::fs::write(&path, serde_json::to_vec_pretty(&payload).map_err(storage)?)
        .map_err(storage)?;
    Ok(FeedbackSubmitResult {
        stored: "local".to_owned(),
        path: path.to_string_lossy().into_owned(),
    })
}
