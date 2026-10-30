#![forbid(unsafe_code)]
//! Shared command dispatch used by both the Tauri shell and the HTTP serve carrier.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use serde_json::{json, Value};
use tokio::sync::broadcast;
use zk_contract_agent::AgentHookSink;
use zk_contract_config::{AppSettings, ProviderKind, ToolSettings};
use zk_contract_ipc::{commands, CommandResponse, UiEvent};
use zk_contract_session::Message;
use zk_contract_tool::TurnContext;

use crate::approval::ApprovalBroker;
use crate::events::{emit_kind, now_ms, BridgeSink, EVENT_BUFFER};
use zk_kit_questions::QuestionBroker;

/// One queued turn for a conversation worker.
///
/// One conversation executes at most one turn at a time (docs/05 §4.1); extra
/// sends are queued FIFO instead of racing on the same conversation.
struct TurnJob {
    text: String,
    conversation_id: String,
    workspace_id: String,
    workspace_root: String,
    stream_id: String,
    cancelled: Arc<AtomicBool>,
}

/// Composed runtime + IPC/event state shared by all transports.
pub struct AppState {
    runtime: Arc<zk_composed::Runtime>,
    pub bus: broadcast::Sender<UiEvent>,
    seq: AtomicU64,
    streams: Mutex<HashMap<String, Arc<AtomicBool>>>,
    turns: Mutex<HashMap<String, mpsc::Sender<TurnJob>>>,
    approvals: Arc<ApprovalBroker>,
    questions: Arc<QuestionBroker>,
    agent_config_error: Mutex<Option<String>>,
}

impl AppState {
    /// Build state around an already composed runtime.
    pub fn new(runtime: zk_composed::Runtime) -> Arc<Self> {
        let (bus, _) = broadcast::channel(EVENT_BUFFER);
        let approvals = ApprovalBroker::new();
        let questions = QuestionBroker::new();
        zk_kit_questions::set_global(Arc::clone(&questions));
        zk_kit_session::set_global(runtime.sessions());
        let concrete = Arc::clone(&approvals);
        let gate: Arc<dyn zk_contract_tool::ApprovalGate> = concrete;
        zk_kit_approval::set_global_gate(gate);
        Arc::new(Self {
            runtime: Arc::new(runtime),
            bus,
            seq: AtomicU64::new(1),
            streams: Mutex::new(HashMap::new()),
            turns: Mutex::new(HashMap::new()),
            approvals,
            questions,
            agent_config_error: Mutex::new(None),
        })
    }

    /// Runtime accessor (tests).
    pub fn runtime(&self) -> &Arc<zk_composed::Runtime> {
        &self.runtime
    }

    fn seq(&self) -> &AtomicU64 {
        &self.seq
    }

    /// Record a non-fatal agent-settings startup error for the UI (`meta.get`).
    pub fn record_agent_config_error(&self, error: impl Into<String>) {
        if let Ok(mut slot) = self.agent_config_error.lock() {
            *slot = Some(error.into());
        }
    }

    /// Apply validated tool/approval timeouts to the live approval broker.
    ///
    /// # Errors
    ///
    /// [`ToolSettings::validate`] rejects out-of-range values (UI surfaces it).
    pub fn apply_tool_settings(&self, tools: &ToolSettings) -> Result<(), String> {
        tools
            .validate()
            .map_err(|error| format!("tool settings: {error}"))?;
        self.approvals.set_timeout_ms(tools.approval_timeout());
        Ok(())
    }

    /// The agent-settings startup error, when the persisted settings could not
    /// be applied (the app still boots so the user can fix them).
    #[must_use]
    pub fn agent_config_error(&self) -> Option<String> {
        self.agent_config_error
            .lock()
            .ok()
            .and_then(|slot| slot.clone())
    }
}

/// Desktop hook logger: every lifecycle event (including usage) is echoed as
/// JSON to stderr; `agent.hooks.log` additionally writes JSONL inside
/// `zk-agent-rig`. Hosts without a UI bridge still get observable hooks.
///
/// Defense in depth: raw tool arguments are stripped here as well, even though
/// the agent hook layer already redacts them before external sinks.
struct DesktopHookSink;

impl AgentHookSink for DesktopHookSink {
    fn on_agent_hook(&self, event: &Value) {
        let redacted = redact_hook_event(event);
        match serde_json::to_string(&redacted) {
            Ok(line) => eprintln!("zk-desktop agent hook: {line}"),
            Err(error) => eprintln!("zk-desktop agent hook: serialize failed: {error}"),
        }
    }
}

/// Strip sensitive payloads from a hook event before logging it.
pub(crate) fn redact_hook_event(event: &Value) -> Value {
    let mut value = event.clone();
    if value.get("hook").and_then(Value::as_str) == Some("tool_started") {
        if let Some(object) = value.as_object_mut() {
            object.insert("args".to_owned(), Value::String("[redacted]".to_owned()));
        }
    }
    value
}

/// Same settings-file resolution as `zk-store-settings-json`.
fn settings_file_path() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("ZEOKEEL_HOME") {
        if !home.trim().is_empty() {
            return Some(PathBuf::from(home).join("settings.json"));
        }
    }
    dirs::data_local_dir().map(|base| base.join("zeokeel").join("settings.json"))
}

fn settings_file_exists() -> bool {
    settings_file_path()
        .map(|path| path.is_file())
        .unwrap_or(false)
}

/// Publish persisted agent settings only when they were explicitly saved and
/// differ from the built-in defaults, so a fresh install (or default-only
/// settings) cannot override `ZEOKEEL_AGENT_*` env fallbacks.
pub(crate) fn should_publish_agent_settings(
    settings: &AppSettings,
    settings_file_exists: bool,
) -> bool {
    settings_file_exists && settings.agent != zk_contract_config::AgentSettings::default()
}

/// Same precedence rule as agent settings: a fresh install or default-only
/// file must not override `ZEOKEEL_TOOL_TIMEOUT_MS`/`ZEOKEEL_APPROVAL_TIMEOUT_MS`.
pub(crate) fn should_publish_tool_settings(
    settings: &AppSettings,
    settings_file_exists: bool,
) -> bool {
    settings_file_exists && settings.tools != ToolSettings::default()
}

/// Validate and publish `AppSettings.agent` to the contract registry the agent
/// runtime reads each turn; install/remove the global hook sink accordingly.
pub(crate) fn apply_agent_settings(settings: &AppSettings) -> Result<(), String> {
    settings
        .agent
        .validate()
        .map_err(|error| format!("agent settings: {error}"))?;
    zk_contract_agent::set_default_agent_settings(&settings.agent)
        .map_err(|error| format!("agent settings: {error}"))?;
    if settings.agent.hooks.enabled {
        zk_contract_agent::set_agent_hook_sink(Some(Arc::new(DesktopHookSink)));
    } else {
        zk_contract_agent::set_agent_hook_sink(None);
    }
    Ok(())
}

/// Boot path: load persisted settings and publish the agent section only when
/// the settings file was explicitly saved with non-default agent settings.
///
/// On a fresh install nothing is published, so `zk-agent-rig` keeps its
/// `ZEOKEEL_AGENT_*` environment fallback (dev/e2e flows).
pub(crate) fn apply_persisted_settings(
    runtime: &zk_composed::Runtime,
) -> Result<Option<ToolSettings>, String> {
    let settings = runtime
        .settings()
        .load()
        .map_err(|error| format!("settings load: {error}"))?;
    let file_exists = settings_file_exists();
    if should_publish_agent_settings(&settings, file_exists) {
        apply_agent_settings(&settings)?;
    }
    if should_publish_tool_settings(&settings, file_exists) {
        zk_contract_config::set_runtime_settings(&settings)
            .map_err(|error| format!("settings publish: {error}"))?;
        return Ok(Some(settings.tools));
    }
    Ok(None)
}

/// Dispatch one command. Never panics; all failures become `CommandResponse::err`.
pub fn dispatch(state: &Arc<AppState>, name: &str, payload: &Value) -> CommandResponse {
    match name {
        commands::META_GET => CommandResponse::ok(json!({
            "edition": zk_composed::EDITION,
            "packages": zk_composed::PACKAGES,
            "version": env!("CARGO_PKG_VERSION"),
            "agent_config_error": state.agent_config_error(),
        })),
        commands::WORKSPACE_LIST => match state.runtime.sessions().list_workspaces() {
            Ok(list) => CommandResponse::ok(json!(list)),
            Err(error) => session_err(error),
        },
        commands::WORKSPACE_CREATE => {
            let (Some(path), title) = (
                payload.get("path").and_then(Value::as_str),
                payload.get("title").and_then(Value::as_str).unwrap_or(""),
            ) else {
                return CommandResponse::err("missing path");
            };
            let expanded = if path == "~" {
                match dirs::home_dir() {
                    Some(home) => home.to_string_lossy().to_string(),
                    None => {
                        return CommandResponse::err(
                            "workspace/path-unavailable: home directory unavailable",
                        );
                    }
                }
            } else {
                path.to_owned()
            };
            match state.runtime.sessions().create_workspace(&expanded, title) {
                Ok(workspace) => CommandResponse::ok(json!(workspace)),
                Err(error) => session_err(error),
            }
        }
        commands::WORKSPACE_RENAME => {
            let (Some(workspace_id), Some(title)) = (
                payload.get("workspace_id").and_then(Value::as_str),
                payload.get("title").and_then(Value::as_str),
            ) else {
                return CommandResponse::err("missing workspace_id/title");
            };
            match state
                .runtime
                .sessions()
                .rename_workspace(workspace_id, title)
            {
                Ok(workspace) => CommandResponse::ok(json!(workspace)),
                Err(error) => session_err(error),
            }
        }
        commands::WORKSPACE_SET_DEFAULT => {
            let Some(workspace_id) = payload.get("workspace_id").and_then(Value::as_str) else {
                return CommandResponse::err("missing workspace_id");
            };
            match state.runtime.sessions().set_default_workspace(workspace_id) {
                Ok(list) => CommandResponse::ok(json!(list)),
                Err(error) => session_err(error),
            }
        }
        commands::WORKSPACE_REMOVE => {
            let Some(workspace_id) = payload.get("workspace_id").and_then(Value::as_str) else {
                return CommandResponse::err("missing workspace_id");
            };
            let reassign = payload.get("reassign_to").and_then(Value::as_str);
            let confirm = payload
                .get("confirm")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            match state
                .runtime
                .sessions()
                .remove_workspace(workspace_id, reassign, confirm)
            {
                Ok(result) => CommandResponse::ok(json!(result)),
                Err(error) => session_err(error),
            }
        }
        commands::SESSION_CREATE => {
            let title = payload
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("新会话");
            let Some(workspace_id) = payload.get("workspace_id").and_then(Value::as_str) else {
                return CommandResponse::err("session/workspace-required: missing workspace_id");
            };
            match state
                .runtime
                .sessions()
                .create_conversation_in(workspace_id, title.trim())
            {
                Ok(conversation) => CommandResponse::ok(json!(conversation)),
                Err(error) => session_err(error),
            }
        }
        commands::SESSION_LIST => {
            let Some(workspace_id) = payload.get("workspace_id").and_then(Value::as_str) else {
                return CommandResponse::err("session/workspace-required: missing workspace_id");
            };
            match state.runtime.sessions().list_conversations_in(workspace_id) {
                Ok(list) => CommandResponse::ok(json!(list)),
                Err(error) => session_err(error),
            }
        }
        commands::SESSION_MESSAGES => {
            let (Some(conversation_id), Some(workspace_id)) = (
                payload.get("conversation_id").and_then(Value::as_str),
                payload.get("workspace_id").and_then(Value::as_str),
            ) else {
                return CommandResponse::err("missing workspace_id/conversation_id");
            };
            match state
                .runtime
                .sessions()
                .messages_in(workspace_id, conversation_id)
            {
                Ok(messages) => CommandResponse::ok(json!(messages)),
                Err(error) => session_err(error),
            }
        }
        commands::SETTINGS_GET => match state.runtime.settings().load() {
            Ok(settings) => CommandResponse::ok(masked_settings(&settings)),
            Err(error) => CommandResponse::err(error.to_string()),
        },
        commands::SETTINGS_SET => {
            let Some(value) = payload.get("settings") else {
                return CommandResponse::err("missing settings");
            };
            let mut settings: AppSettings = match serde_json::from_value(value.clone()) {
                Ok(settings) => settings,
                Err(error) => return CommandResponse::err(format!("invalid settings: {error}")),
            };
            if settings.provider.api_key.trim().is_empty() {
                // The UI never receives the stored key (masked by settings.get).
                // An empty field means "keep the existing secret".
                if let Ok(existing) = state.runtime.settings().load() {
                    settings.provider.api_key = existing.provider.api_key;
                }
            }
            if let Err(error) = validate_settings(&settings) {
                return CommandResponse::err(error);
            }
            // Apply agent + tool settings first: validation/runtime failures
            // must reach the UI instead of persisting settings that never take
            // effect.
            if let Err(error) = apply_agent_settings(&settings) {
                return CommandResponse::err(error);
            }
            if let Err(error) = zk_contract_config::set_runtime_settings(&settings) {
                return CommandResponse::err(format!("settings publish: {error}"));
            }
            if let Err(error) = state.apply_tool_settings(&settings.tools) {
                return CommandResponse::err(error);
            }
            // Apply provider hot-swap: a configure failure must reach the UI
            // instead of being discarded after persisting an unusable provider.
            let provider = match settings.provider.kind {
                ProviderKind::Mock => None,
                ProviderKind::OpenAiCompatible => Some("openai"),
                ProviderKind::Ollama => Some("ollama"),
            };
            if let Some(provider) = provider {
                if let Err(error) = state.runtime.configure_llm(
                    &settings.provider.base_url,
                    &settings.provider.api_key,
                    &settings.provider.model,
                    provider,
                ) {
                    return CommandResponse::err(format!("provider configure failed: {error}"));
                }
            }
            match state.runtime.settings().save(&settings) {
                Ok(()) => CommandResponse::ok(masked_settings(&settings)),
                Err(error) => CommandResponse::err(error.to_string()),
            }
        }
        commands::WORKSPACE_FILES => {
            let Some(workspace_id) = payload.get("workspace_id").and_then(Value::as_str) else {
                return CommandResponse::err("missing workspace_id");
            };
            match state.runtime.sessions().list_workspaces() {
                Ok(workspaces) => match workspaces
                    .iter()
                    .find(|workspace| workspace.id == workspace_id)
                {
                    Some(workspace) => match list_workspace_files(&workspace.path) {
                        Ok(items) => CommandResponse::ok(json!({ "items": items })),
                        Err(error) => CommandResponse::err(format!("workspace/files: {error}")),
                    },
                    None => CommandResponse::err("workspace/not-found"),
                },
                Err(error) => CommandResponse::err(error.to_string()),
            }
        }
        commands::AUTH_CONFIGURE => {
            let (Some(base_url), Some(access_token)) = (
                payload.get("server_url").and_then(Value::as_str),
                payload.get("access_token").and_then(Value::as_str),
            ) else {
                return CommandResponse::err("missing server_url/access_token");
            };
            let model = payload.get("model").and_then(Value::as_str).unwrap_or("");
            // Shared config feeds the gateway credentials/token refresh paths ...
            zk_composed::configure_llm(base_url, access_token, model);
            // ... while this rebuilds the already-constructed rig model so login and
            // refresh take effect without restarting the app (ADR-064 hot-swap).
            match state
                .runtime
                .configure_llm(base_url, access_token, model, "gateway")
            {
                Ok(()) => CommandResponse::ok(json!({ "configured": true })),
                Err(error) => CommandResponse::err(format!("auth/configure: {error}")),
            }
        }
        commands::AUTH_LOGOUT => {
            // Clear the enterprise gateway credentials held by the composed runtime.
            // The local session (tokens/server URL) is owned by the UI layer; here we
            // only make sure no in-flight turn can keep using the old credentials.
            zk_composed::configure_llm("", "", "");
            // Reset the already-built rig model to the boot placeholder so a
            // logged-out agent cannot keep using the old gateway credentials.
            let _ = state.runtime.configure_llm("", "", "", "gateway");
            if let Ok(streams) = state.streams.lock() {
                for cancelled in streams.values() {
                    cancelled.store(true, Ordering::SeqCst);
                }
            }
            state.runtime.cancel_agent();
            CommandResponse::ok(json!({ "logged_out": true }))
        }
        commands::UPDATE_CHECK => {
            let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
            match zk_composed::check_update(env!("CARGO_PKG_VERSION"), &platform) {
                Ok(release) => {
                    CommandResponse::ok(json!({ "release": release, "platform": platform }))
                }
                Err(error) => CommandResponse::err(format!("update/check: {error}")),
            }
        }
        commands::APP_REQUEST => app_request(payload),
        commands::SKILLS_INSTALL => install_skill(payload),
        commands::SKILLS_INSTALLED => installed_skills(),
        commands::SKILLS_REMOVE => remove_skill(payload),
        commands::OAUTH_TOKEN_SAVE => oauth_save(payload),
        commands::OAUTH_TOKEN_LOAD => oauth_load(payload),
        commands::OAUTH_TOKEN_CLEAR => oauth_clear(payload),
        commands::CHAT_SEND => chat_send(state, payload),
        commands::CHAT_CANCEL => chat_cancel(state, payload),
        commands::APPROVAL_DECIDE => {
            let (Some(approval_id), Some(decision)) = (
                payload.get("approval_id").and_then(Value::as_str),
                payload.get("decision").and_then(Value::as_str),
            ) else {
                return CommandResponse::err("missing approval_id/decision");
            };
            if state.approvals.resolve(approval_id, decision == "approved") {
                CommandResponse::ok(json!({ "resolved": true }))
            } else {
                CommandResponse::err("approval/pending-not-found")
            }
        }
        commands::QUESTION_ANSWER => {
            let (Some(question_id), Some(answers)) = (
                payload.get("question_id").and_then(Value::as_str),
                payload.get("answers").cloned(),
            ) else {
                return CommandResponse::err("missing question_id/answers");
            };
            if state.questions.answer(question_id, answers) {
                CommandResponse::ok(json!({ "answered": true }))
            } else {
                CommandResponse::err("question/pending-not-found")
            }
        }
        other => CommandResponse::err(format!("unknown command: {other}")),
    }
}

/// Bounded, read-only file listing under one workspace root.
fn list_workspace_files(root: &str) -> Result<Vec<Value>, String> {
    const MAX_ENTRIES: usize = 300;
    let mut items = Vec::new();
    let mut stack = vec![(std::path::PathBuf::from(root), 0_usize)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > 2 || items.len() >= MAX_ENTRIES {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut children: Vec<_> = entries.filter_map(Result::ok).collect();
        children.sort_by_key(|entry| entry.file_name());
        for entry in children {
            if items.len() >= MAX_ENTRIES {
                break;
            }
            let path = entry.path();
            let rel = path
                .strip_prefix(root)
                .map(|value| value.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            if rel.is_empty() || rel.starts_with(".git") || rel.contains("/.git/") {
                continue;
            }
            let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
            items.push(
                json!({ "path": rel, "depth": depth, "kind": if is_dir { "dir" } else { "file" } }),
            );
            if is_dir {
                stack.push((path, depth.saturating_add(1)));
            }
        }
    }
    items.sort_by(|a, b| {
        let a_path = a.get("path").and_then(Value::as_str).unwrap_or("");
        let b_path = b.get("path").and_then(Value::as_str).unwrap_or("");
        a_path.cmp(b_path)
    });
    Ok(items)
}

fn app_request(payload: &Value) -> CommandResponse {
    let Some(app_id) = payload.get("app_id").and_then(Value::as_str) else {
        return CommandResponse::err("missing app_id");
    };
    if app_id.is_empty()
        || app_id.len() > 64
        || !app_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return CommandResponse::err("APP_BAD_REQUEST");
    }
    let Some((base_url, token)) = zk_composed::gateway_credentials() else {
        return CommandResponse::err("AUTH_REQUIRED");
    };
    let request = json!({
        "abi": "zeokeel-app/1",
        "app_id": app_id,
        "method": payload.get("method").and_then(Value::as_str).unwrap_or("GET"),
        "path": payload.get("path").and_then(Value::as_str).unwrap_or("/"),
        "query": payload.get("query").and_then(Value::as_str).unwrap_or(""),
        "headers": payload.get("headers").cloned().unwrap_or(json!({})),
        "body_b64": payload.get("body_b64").and_then(Value::as_str).unwrap_or(""),
        "request_id": payload.get("request_id").and_then(Value::as_str).unwrap_or("host"),
    });
    let url = format!(
        "{}/api/client/v1/apps/{app_id}/request",
        base_url.trim_end_matches('/')
    );
    let client = reqwest::blocking::Client::new();
    match client.post(url).bearer_auth(token).json(&request).send() {
        Ok(response) => match response.error_for_status() {
            Ok(ok) => match ok.json::<Value>() {
                Ok(value) => CommandResponse::ok(value),
                Err(error) => CommandResponse::err(format!("APP_BAD_RESPONSE: {error}")),
            },
            Err(error) => CommandResponse::err(format!("APP_UNAVAILABLE: {error}")),
        },
        Err(error) => CommandResponse::err(format!("APP_UNAVAILABLE: {error}")),
    }
}

fn skill_root() -> Result<std::path::PathBuf, String> {
    let base = match std::env::var("ZEOKEEL_HOME") {
        Ok(value) => std::path::PathBuf::from(value),
        Err(_) => dirs::home_dir()
            .ok_or_else(|| "home directory unavailable".to_owned())?
            .join(".zeokeel"),
    };
    Ok(base.join("skills"))
}

fn safe_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value != "."
        && value != ".."
        && !value.starts_with('.')
        && !value.contains(':')
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn install_skill(payload: &Value) -> CommandResponse {
    use base64::Engine;
    use std::io::Read;
    let (Some(name), Some(version), Some(encoded)) = (
        payload.get("name").and_then(Value::as_str),
        payload.get("version").and_then(Value::as_str),
        payload.get("archive_base64").and_then(Value::as_str),
    ) else {
        return CommandResponse::err("missing name/version/archive_base64");
    };
    if !safe_component(name) || !safe_component(version) {
        return CommandResponse::err("SKILL_VALIDATION");
    }
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded) else {
        return CommandResponse::err("SKILL_ZIP_INVALID");
    };
    if bytes.len() > 16 * 1024 * 1024 {
        return CommandResponse::err("SKILL_ARCHIVE_TOO_LARGE");
    }
    let Ok(mut archive) = zip::ZipArchive::new(std::io::Cursor::new(bytes)) else {
        return CommandResponse::err("SKILL_ZIP_INVALID");
    };
    let root = match skill_root() {
        Ok(root) => root,
        Err(error) => return CommandResponse::err(error),
    };
    let target = root.join(name).join(version);
    if target.exists() {
        return CommandResponse::err("CONFLICT");
    }
    let temp = root.join(format!(".tmp-{name}-{version}-{}", uuid::Uuid::new_v4()));
    if let Err(error) = std::fs::create_dir_all(&temp) {
        return CommandResponse::err(format!("skills install: {error}"));
    }
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    let mut total = 0_u64;
    for index in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(index) else {
            let _ = std::fs::remove_dir_all(&temp);
            return CommandResponse::err("SKILL_ZIP_INVALID");
        };
        let Some(relative) = entry.enclosed_name().map(|path| path.to_path_buf()) else {
            let _ = std::fs::remove_dir_all(&temp);
            return CommandResponse::err("SKILL_ENTRY_FORBIDDEN");
        };
        let mode = entry.unix_mode().unwrap_or(0o100644);
        let kind = mode & 0o170000;
        if kind != 0 && kind != 0o100000 && kind != 0o040000 {
            let _ = std::fs::remove_dir_all(&temp);
            return CommandResponse::err("SKILL_ENTRY_FORBIDDEN");
        }
        let destination = temp.join(&relative);
        if entry.is_dir() {
            if let Err(error) = std::fs::create_dir_all(&destination) {
                let _ = std::fs::remove_dir_all(&temp);
                return CommandResponse::err(error.to_string());
            }
            continue;
        }
        total = total.saturating_add(entry.size());
        if total > 64 * 1024 * 1024 {
            let _ = std::fs::remove_dir_all(&temp);
            return CommandResponse::err("SKILL_ARCHIVE_TOO_LARGE");
        }
        let mut content = Vec::new();
        if let Err(error) = entry.read_to_end(&mut content) {
            let _ = std::fs::remove_dir_all(&temp);
            return CommandResponse::err(format!("SKILL_ZIP_INVALID: {error}"));
        }
        if let Some(parent) = destination.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                let _ = std::fs::remove_dir_all(&temp);
                return CommandResponse::err(error.to_string());
            }
        }
        if let Err(error) = std::fs::write(&destination, &content) {
            let _ = std::fs::remove_dir_all(&temp);
            return CommandResponse::err(error.to_string());
        }
        entries.push((relative.to_string_lossy().replace('\\', "/"), content));
    }
    let tree = zk_kit_content::tree_sha256(entries);
    if let Some(parent) = target.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            let _ = std::fs::remove_dir_all(&temp);
            return CommandResponse::err(error.to_string());
        }
    }
    if let Err(error) = std::fs::rename(&temp, &target) {
        let _ = std::fs::remove_dir_all(&temp);
        return CommandResponse::err(format!("skills install: {error}"));
    }
    CommandResponse::ok(
        json!({ "installed": true, "tree_sha256": tree, "path": target.to_string_lossy() }),
    )
}

fn dev_oauth_fallback() -> bool {
    std::env::var("ZEOKEEL_DEV_KEYCHAIN_FALLBACK")
        .map(|value| value == "1")
        .unwrap_or(false)
}

fn dev_oauth_path() -> Result<std::path::PathBuf, String> {
    let base = match std::env::var("ZEOKEEL_HOME") {
        Ok(value) => std::path::PathBuf::from(value),
        Err(_) => dirs::home_dir()
            .ok_or_else(|| "home directory unavailable".to_owned())?
            .join(".zeokeel"),
    };
    Ok(base.join("oauth-dev.json"))
}

fn read_dev_oauth() -> serde_json::Map<String, Value> {
    let Ok(path) = dev_oauth_path() else {
        return serde_json::Map::new();
    };
    let Ok(raw) = std::fs::read_to_string(path) else {
        return serde_json::Map::new();
    };
    match serde_json::from_str::<Value>(&raw) {
        Ok(Value::Object(map)) => map,
        _ => serde_json::Map::new(),
    }
}

fn write_dev_oauth(map: &serde_json::Map<String, Value>) -> Result<(), String> {
    let path = dev_oauth_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&path, Value::Object(map.clone()).to_string())
        .map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn oauth_entry(connector_id: &str) -> Result<keyring::Entry, String> {
    if connector_id.is_empty()
        || connector_id.len() > 128
        || !connector_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err("OAUTH_VALIDATION".to_owned());
    }
    keyring::Entry::new("zeokeel-connector-oauth", connector_id)
        .map_err(|error| format!("keychain: {error}"))
}

fn oauth_save(payload: &Value) -> CommandResponse {
    let (Some(connector_id), Some(tokens)) = (
        payload.get("connector_id").and_then(Value::as_str),
        payload.get("tokens"),
    ) else {
        return CommandResponse::err("missing connector_id/tokens");
    };
    if !tokens.is_object() {
        return CommandResponse::err("OAUTH_VALIDATION");
    }
    let entry = match oauth_entry(connector_id) {
        Ok(entry) => entry,
        Err(_) if dev_oauth_fallback() => {
            let mut map = read_dev_oauth();
            map.insert(connector_id.to_owned(), tokens.clone());
            return match write_dev_oauth(&map) {
                Ok(()) => CommandResponse::ok(json!({ "saved": true, "fallback": true })),
                Err(error) => CommandResponse::err(format!("keychain fallback: {error}")),
            };
        }
        Err(error) => return CommandResponse::err(error),
    };
    match entry.set_password(&tokens.to_string()) {
        Ok(()) => CommandResponse::ok(json!({ "saved": true })),
        Err(_error) if dev_oauth_fallback() => {
            let mut map = read_dev_oauth();
            map.insert(connector_id.to_owned(), tokens.clone());
            match write_dev_oauth(&map) {
                Ok(()) => CommandResponse::ok(json!({ "saved": true, "fallback": true })),
                Err(error) => CommandResponse::err(format!("keychain fallback: {error}")),
            }
        }
        Err(error) => CommandResponse::err(format!("keychain: {error}")),
    }
}

fn oauth_load(payload: &Value) -> CommandResponse {
    let Some(connector_id) = payload.get("connector_id").and_then(Value::as_str) else {
        return CommandResponse::err("missing connector_id");
    };
    let entry = match oauth_entry(connector_id) {
        Ok(entry) => entry,
        Err(_) if dev_oauth_fallback() => {
            return CommandResponse::ok(
                json!({ "tokens": read_dev_oauth().get(connector_id).map(Value::to_string) }),
            );
        }
        Err(error) => return CommandResponse::err(error),
    };
    match entry.get_password() {
        Ok(raw) => CommandResponse::ok(json!({ "tokens": raw })),
        Err(_) if dev_oauth_fallback() => CommandResponse::ok(
            json!({ "tokens": read_dev_oauth().get(connector_id).map(Value::to_string) }),
        ),
        Err(keyring::Error::NoEntry) => CommandResponse::ok(json!({ "tokens": null })),
        Err(error) => CommandResponse::err(format!("keychain: {error}")),
    }
}

fn oauth_clear(payload: &Value) -> CommandResponse {
    let Some(connector_id) = payload.get("connector_id").and_then(Value::as_str) else {
        return CommandResponse::err("missing connector_id");
    };
    let entry = match oauth_entry(connector_id) {
        Ok(entry) => entry,
        Err(_) if dev_oauth_fallback() => {
            let mut map = read_dev_oauth();
            map.remove(connector_id);
            return match write_dev_oauth(&map) {
                Ok(()) => CommandResponse::ok(json!({ "cleared": true, "fallback": true })),
                Err(error) => CommandResponse::err(format!("keychain fallback: {error}")),
            };
        }
        Err(error) => return CommandResponse::err(error),
    };
    match entry.delete_credential() {
        Ok(()) => CommandResponse::ok(json!({ "cleared": true })),
        Err(_) if dev_oauth_fallback() => {
            let mut map = read_dev_oauth();
            map.remove(connector_id);
            match write_dev_oauth(&map) {
                Ok(()) => CommandResponse::ok(json!({ "cleared": true, "fallback": true })),
                Err(error) => CommandResponse::err(format!("keychain fallback: {error}")),
            }
        }
        Err(keyring::Error::NoEntry) => CommandResponse::ok(json!({ "cleared": true })),
        Err(error) => CommandResponse::err(format!("keychain: {error}")),
    }
}

fn remove_skill(payload: &Value) -> CommandResponse {
    let Some(name) = payload.get("name").and_then(Value::as_str) else {
        return CommandResponse::err("missing name");
    };
    if !safe_component(name) {
        return CommandResponse::err("SKILL_VALIDATION");
    }
    let root = match skill_root() {
        Ok(root) => root,
        Err(error) => return CommandResponse::err(error),
    };
    let target = root.join(name);
    if target.exists() {
        if let Err(error) = std::fs::remove_dir_all(&target) {
            return CommandResponse::err(format!("skills remove: {error}"));
        }
    }
    CommandResponse::ok(json!({ "removed": true }))
}

fn installed_skills() -> CommandResponse {
    let root = match skill_root() {
        Ok(root) => root,
        Err(error) => return CommandResponse::err(error),
    };
    let mut items = Vec::new();
    if let Ok(names) = std::fs::read_dir(&root) {
        for name in names.filter_map(Result::ok) {
            if !name.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                continue;
            }
            let name_value = name.file_name().to_string_lossy().to_string();
            if let Ok(versions) = std::fs::read_dir(name.path()) {
                for version in versions.filter_map(Result::ok) {
                    if version
                        .file_type()
                        .map(|kind| kind.is_dir())
                        .unwrap_or(false)
                    {
                        items.push(json!({ "name": name_value, "version": version.file_name().to_string_lossy() }));
                    }
                }
            }
        }
    }
    CommandResponse::ok(json!({ "items": items }))
}

fn masked_settings(settings: &AppSettings) -> Value {
    let mut value = json!(settings);
    if let Some(provider) = value.get_mut("provider").and_then(Value::as_object_mut) {
        provider.insert("api_key".to_owned(), Value::String(String::new()));
    }
    value
}

fn session_err(error: zk_contract_session::SessionError) -> CommandResponse {
    CommandResponse::err(format!("{}: {error}", error.code()))
}

fn validate_settings(settings: &AppSettings) -> Result<(), String> {
    if settings.schema != 1 {
        return Err(format!("unsupported schema {}", settings.schema));
    }
    if !matches!(settings.theme.as_str(), "auto" | "light" | "dark") {
        return Err(format!("invalid theme {}", settings.theme));
    }
    if !matches!(settings.locale.as_str(), "zh-CN" | "en-US") {
        return Err(format!("invalid locale {}", settings.locale));
    }
    if let Err(error) = settings.tools.validate() {
        return Err(error.to_string());
    }
    if settings.provider.kind != ProviderKind::Mock {
        let url = settings.provider.base_url.trim();
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err("base_url must start with http:// or https://".to_owned());
        }
        if settings.provider.model.trim().is_empty() {
            return Err("model is required".to_owned());
        }
    }
    Ok(())
}

fn chat_send(state: &Arc<AppState>, payload: &Value) -> CommandResponse {
    let Some(text) = payload.get("text").and_then(Value::as_str).map(str::trim) else {
        return CommandResponse::err("missing text");
    };
    if text.is_empty() {
        return CommandResponse::err("missing text");
    }
    let sessions = state.runtime.sessions();
    let Some(workspace_id) = payload.get("workspace_id").and_then(Value::as_str) else {
        return CommandResponse::err("session/workspace-required: missing workspace_id");
    };
    let Some(conversation_id) = payload.get("conversation_id").and_then(Value::as_str) else {
        return CommandResponse::err(format!(
            "missing conversation_id in workspace {workspace_id}"
        ));
    };
    let workspace_id = workspace_id.to_owned();
    let conversation_id = conversation_id.to_owned();
    let list = match sessions.list_conversations_in(&workspace_id) {
        Ok(list) => list,
        Err(error) => return CommandResponse::err(error.to_string()),
    };
    if !list
        .iter()
        .any(|conversation| conversation.id == conversation_id)
    {
        return CommandResponse::err(format!("session/workspace-mismatch: {conversation_id}"));
    }
    let workspaces = match sessions.list_workspaces() {
        Ok(list) => list,
        Err(error) => return CommandResponse::err(error.to_string()),
    };
    let Some(workspace) = workspaces
        .into_iter()
        .find(|workspace| workspace.id == workspace_id)
    else {
        return CommandResponse::err(format!("workspace/not-found: {workspace_id}"));
    };
    let workspace_root = (workspace_id.clone(), workspace.path);
    let user_message = Message {
        id: uuid::Uuid::new_v4().to_string(),
        conversation_id: conversation_id.clone(),
        role: "user".to_owned(),
        content: text.to_owned(),
        created_at_ms: now_ms(),
        status: "done".to_owned(),
    };
    if let Err(error) = sessions.append_message(&user_message) {
        return session_err(error);
    }
    let stream_id = uuid::Uuid::new_v4().to_string();
    let cancelled = Arc::new(AtomicBool::new(false));
    if let Ok(mut streams) = state.streams.lock() {
        streams.insert(stream_id.clone(), Arc::clone(&cancelled));
    }

    let job = TurnJob {
        text: text.to_owned(),
        conversation_id: conversation_id.clone(),
        workspace_id: workspace_id.clone(),
        workspace_root: workspace_root.1,
        stream_id: stream_id.clone(),
        cancelled,
    };
    let sender = turn_sender(state, &conversation_id);
    if sender.send(job).is_err() {
        if let Ok(mut streams) = state.streams.lock() {
            streams.remove(&stream_id);
        }
        return CommandResponse::err("session/busy: turn worker unavailable; retry");
    }

    CommandResponse::ok(json!({
        "stream_id": stream_id,
        "conversation_id": conversation_id,
        "workspace_id": workspace_id,
    }))
}

/// Return the per-conversation worker sender, spawning the FIFO worker on demand.
fn turn_sender(state: &Arc<AppState>, conversation_id: &str) -> mpsc::Sender<TurnJob> {
    let mut turns = state
        .turns
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(sender) = turns.get(conversation_id) {
        return sender.clone();
    }
    let (sender, receiver) = mpsc::channel::<TurnJob>();
    let worker_state = Arc::clone(state);
    std::thread::spawn(move || {
        while let Ok(job) = receiver.recv() {
            run_turn(&worker_state, &job);
        }
    });
    turns.insert(conversation_id.to_owned(), sender.clone());
    sender
}

/// Execute one queued turn (runs on that conversation's worker thread).
fn run_turn(state: &Arc<AppState>, job: &TurnJob) {
    if job.cancelled.load(Ordering::SeqCst) {
        // Cancelled while queued: the cancel command already emitted the event.
        if let Ok(mut streams) = state.streams.lock() {
            streams.remove(&job.stream_id);
        }
        return;
    }
    let seq = Arc::new(AtomicU64::new(1));
    let sink: Arc<dyn zk_contract_agent::EventSink> = Arc::new(BridgeSink {
        bus: state.bus.clone(),
        stream_id: job.stream_id.clone(),
        seq: Arc::clone(&seq),
        cancelled: Arc::clone(&job.cancelled),
        sessions: state.runtime.sessions(),
        conversation_id: job.conversation_id.clone(),
    });
    let context = TurnContext {
        conversation_id: job.conversation_id.clone(),
        workspace_id: job.workspace_id.clone(),
        workspace_root: job.workspace_root.clone(),
    };
    let result = state
        .runtime
        .run_stream_in_shared(&context, &job.stream_id, &job.text, sink);
    match result {
        Ok(reply) => {
            let message = Message {
                id: uuid::Uuid::new_v4().to_string(),
                conversation_id: job.conversation_id.clone(),
                role: "assistant".to_owned(),
                content: reply.text,
                created_at_ms: now_ms(),
                status: if job.cancelled.load(Ordering::SeqCst) {
                    "cancelled"
                } else {
                    "done"
                }
                .to_owned(),
            };
            let _ = state.runtime.sessions().append_message(&message);
        }
        Err(error) => {
            let message = Message {
                id: uuid::Uuid::new_v4().to_string(),
                conversation_id: job.conversation_id.clone(),
                role: "assistant".to_owned(),
                content: format!("错误：{error}"),
                created_at_ms: now_ms(),
                status: "error".to_owned(),
            };
            let _ = state.runtime.sessions().append_message(&message);
        }
    }
    if let Ok(mut streams) = state.streams.lock() {
        streams.remove(&job.stream_id);
    }
}

fn chat_cancel(state: &Arc<AppState>, payload: &Value) -> CommandResponse {
    let Some(stream_id) = payload.get("stream_id").and_then(Value::as_str) else {
        return CommandResponse::err("missing stream_id");
    };
    let flag = state
        .streams
        .lock()
        .ok()
        .and_then(|streams| streams.get(stream_id).cloned());
    let Some(flag) = flag else {
        return CommandResponse::err(format!("unknown stream: {stream_id}"));
    };
    flag.store(true, Ordering::SeqCst);
    state.runtime.cancel_agent();
    emit_kind(
        &state.bus,
        stream_id,
        state.seq(),
        "agent",
        json!({ "type": "turn_cancelled" }),
    );
    CommandResponse::ok(json!({ "cancelled": true }))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only
    use base64::Engine;
    use std::io::Write;

    fn zip_bytes(entries: &[(&str, &[u8], Option<u32>)]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, content, mode) in entries {
            let mut options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            if let Some(mode) = mode {
                options = options.unix_permissions(*mode);
            }
            writer.start_file(*name, options).unwrap();
            writer.write_all(content).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn install(name: &str, version: &str, bytes: &[u8]) -> serde_json::Value {
        let payload = serde_json::json!({
            "name": name,
            "version": version,
            "archive_base64": base64::engine::general_purpose::STANDARD.encode(bytes),
        });
        let response = super::install_skill(&payload);
        serde_json::to_value(response).unwrap()
    }

    fn zip_symlink_bytes() -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("SKILL.md", options).unwrap();
        writer.write_all(b"# link\n").unwrap();
        writer.add_symlink("link", "target", options).unwrap();
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn boot_publishes_only_explicitly_saved_non_default_tool_settings() {
        let default_settings = zk_contract_config::AppSettings::default();
        assert!(!super::should_publish_tool_settings(
            &default_settings,
            false
        ));
        assert!(!super::should_publish_tool_settings(
            &default_settings,
            true
        ));

        let mut custom = zk_contract_config::AppSettings::default();
        custom.tools.tool_timeout_ms = 5_000;
        assert!(!super::should_publish_tool_settings(&custom, false));
        assert!(super::should_publish_tool_settings(&custom, true));

        let mut invalid = zk_contract_config::AppSettings::default();
        invalid.tools.tool_timeout_ms = 10;
        assert!(super::validate_settings(&invalid).is_err());
        invalid.tools.tool_timeout_ms = 120_000;
        invalid.tools.approval_timeout_ms = 2_000_000;
        assert!(super::validate_settings(&invalid).is_err());
        assert!(super::validate_settings(&zk_contract_config::AppSettings::default()).is_ok());
    }

    #[test]
    fn boot_publishes_only_explicitly_saved_non_default_agent_settings() {
        let default_settings = zk_contract_config::AppSettings::default();
        // Fresh install / default-only: env fallback must win (nothing published).
        assert!(!super::should_publish_agent_settings(
            &default_settings,
            false
        ));
        assert!(!super::should_publish_agent_settings(
            &default_settings,
            true
        ));

        let mut custom = zk_contract_config::AppSettings::default();
        custom.agent.max_steps = 12;
        // Custom settings only count once the settings file exists.
        assert!(!super::should_publish_agent_settings(&custom, false));
        assert!(super::should_publish_agent_settings(&custom, true));
    }

    #[test]
    fn hook_events_are_redacted_before_logging() {
        let redacted = super::redact_hook_event(&serde_json::json!({
            "hook": "tool_started",
            "turn_id": "t",
            "tool": "bash",
            "args": "{\"command\":\"echo SECRET\"}"
        }));
        assert_eq!(redacted["args"], "[redacted]");
        assert!(!serde_json::to_string(&redacted).unwrap().contains("SECRET"));
        let usage = super::redact_hook_event(&serde_json::json!({
            "hook": "turn_completed",
            "usage": {"input_tokens": 3}
        }));
        assert_eq!(usage["usage"]["input_tokens"], 3);
    }

    #[test]
    fn agent_settings_apply_to_the_runtime_registry_and_validate() {
        let mut settings = zk_contract_config::AppSettings::default();
        settings.agent.completion.temperature = Some(0.25);
        settings.agent.max_steps = 12;
        super::apply_agent_settings(&settings).unwrap();
        let published = zk_contract_agent::default_agent_settings().unwrap();
        assert_eq!(published.completion.temperature, Some(0.25));
        assert_eq!(published.max_steps, 12);

        // Out-of-range settings are rejected and surface as an error (no silent save).
        let mut invalid = settings.clone();
        invalid.agent.completion.temperature = Some(9.0);
        assert!(super::apply_agent_settings(&invalid).is_err());

        super::apply_agent_settings(&zk_contract_config::AppSettings::default()).unwrap();
        zk_contract_agent::set_agent_hook_sink(None);
    }

    #[test]
    fn installs_valid_archive_and_rejects_hostile_entries() {
        let home = std::env::temp_dir().join(format!("zk-skill-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", &home);

        let valid = zip_bytes(&[
            ("SKILL.md", b"# demo\n", None),
            ("scripts/run.sh", b"echo ok\n", None),
        ]);
        let ok = install("demo-skill", "1.0.0", &valid);
        assert_eq!(ok["ok"], serde_json::Value::Bool(true), "{ok}");
        assert_eq!(ok["data"]["tree_sha256"].as_str().unwrap().len(), 64);
        assert!(home.join("skills/demo-skill/1.0.0/SKILL.md").is_file());

        let duplicate = install("demo-skill", "1.0.0", &valid);
        assert_eq!(duplicate["ok"], serde_json::Value::Bool(false));
        assert_eq!(duplicate["error"], "CONFLICT");

        let traversal = zip_bytes(&[("SKILL.md", b"# bad\n", None), ("../evil.txt", b"x", None)]);
        let bad = install("evil-skill", "1.0.0", &traversal);
        assert_eq!(bad["ok"], serde_json::Value::Bool(false), "{bad}");

        let link = install("link-skill", "1.0.0", &zip_symlink_bytes());
        assert_eq!(link["ok"], serde_json::Value::Bool(false), "{link}");

        for hostile in [".", "..", ".hidden", "C:evil", "a/b"] {
            assert!(!super::safe_component(hostile), "{hostile}");
            let response = super::remove_skill(&serde_json::json!({ "name": hostile }));
            assert!(!response.ok, "{hostile}");
        }
        assert!(
            home.join("skills/demo-skill/1.0.0/SKILL.md").is_file(),
            "hostile remove must not delete installed skills"
        );

        let removed = super::remove_skill(&serde_json::json!({ "name": "demo-skill" }));
        assert!(removed.ok);
        assert!(!home.join("skills/demo-skill").exists());
        std::fs::remove_dir_all(&home).unwrap();
    }
}
