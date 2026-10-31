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
use zk_contract_ipc::results::{ChatSendResult, MetaInfo};
use zk_contract_ipc::{commands, CommandResponse, UiEvent};
use zk_contract_session::{FileEntry, FileKind, Message, WorkspaceFilesResult};
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

/// Validate and publish `AppSettings.agent` to the contract registry the agent
/// runtime reads each turn; install/remove the global hook sink accordingly.
pub(crate) fn apply_agent_settings(settings: &AppSettings) -> Result<(), String> {
    settings
        .agent
        .validate()
        .map_err(|error| format!("agent settings: {error}"))?;
    // Only an explicitly non-default agent section is published; a tools-only
    // save (or a reset to defaults) must clear the registry so `ZEOKEEL_AGENT_*`
    // env stays authoritative instead of being pinned by default values.
    if settings.agent == zk_contract_config::AgentSettings::default() {
        zk_contract_agent::clear_default_agent_settings()
            .map_err(|error| format!("agent settings: {error}"))?;
    } else {
        zk_contract_agent::set_default_agent_settings(&settings.agent)
            .map_err(|error| format!("agent settings: {error}"))?;
    }
    if settings.agent.hooks.enabled {
        zk_contract_agent::set_agent_hook_sink(Some(Arc::new(DesktopHookSink)));
    } else {
        zk_contract_agent::set_agent_hook_sink(None);
    }
    Ok(())
}

/// Build the non-secret runtime snapshot for the sections that should be
/// published (only sections that were explicitly saved and are non-default).
///
/// Per-section `Option`s keep the env fallback alive: a tools-only save must
/// not pin agent defaults, and vice versa. Provider config / `api_key` never
/// enter the snapshot.
pub(crate) fn runtime_snapshot_for(
    settings: &AppSettings,
    include_agent: bool,
    include_tools: bool,
) -> zk_contract_config::RuntimeSettings {
    zk_contract_config::RuntimeSettings {
        agent: include_agent
            .then(|| settings.agent.clone())
            .filter(|agent| *agent != zk_contract_config::AgentSettings::default()),
        tools: include_tools
            .then_some(settings.tools)
            .filter(|tools| *tools != ToolSettings::default()),
    }
}

/// Publish the non-secret runtime snapshot (`agent` + `tools` only).
pub(crate) fn publish_runtime_settings(
    snapshot: &zk_contract_config::RuntimeSettings,
) -> Result<(), String> {
    zk_contract_config::set_runtime_settings(snapshot)
        .map_err(|error| format!("settings publish: {error}"))
}

/// Apply `AppSettings.agent.schedule` to the live schedule executor.
///
/// Explicitly saved settings always win — **including the default**
/// `{enabled:false, scan_seconds:10}` — so a user save can disable auto-run even
/// with `ZEOKEEL_ENABLE_SCHEDULE_AUTORUN=1`. The env fallback only applies when
/// no settings file exists (fresh install).
pub(crate) fn apply_schedule_settings(settings: &AppSettings) -> Result<(), String> {
    crate::automation::apply_settings(&settings.agent.schedule)
        .map_err(|error| format!("agent settings: {error}"))
}

/// Boot rule: a settings file means its schedule (even default-valued) is
/// explicit and pins the override; only a fresh install falls back to env.
pub(crate) fn apply_persisted_schedule(
    settings: &AppSettings,
    file_exists: bool,
) -> Result<(), String> {
    if file_exists {
        apply_schedule_settings(settings)
    } else {
        crate::automation::clear_settings_override();
        Ok(())
    }
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
    if zk_composed::EDITION == "personal" {
        // Boot order: env-based capability configuration already ran in
        // `boot_desktop`; explicit saved settings override it here.
        if let Err(error) = apply_embedding_settings(&settings) {
            eprintln!("zk-desktop: embedding settings not applied: {error}");
        }
    }
    // A settings file (even one carrying the default schedule) is explicit:
    // it overrides env. Only a fresh install (no file) uses the env fallback.
    apply_persisted_schedule(&settings, file_exists)?;
    if should_publish_agent_settings(&settings, file_exists) {
        apply_agent_settings(&settings)?;
    }
    let snapshot = runtime_snapshot_for(&settings, file_exists, file_exists);
    if snapshot.agent.is_some() || snapshot.tools.is_some() {
        publish_runtime_settings(&snapshot)?;
    }
    Ok(snapshot.tools)
}

/// Dispatch one command. Never panics; all failures become `CommandResponse::err`.
pub fn dispatch(state: &Arc<AppState>, name: &str, payload: &Value) -> CommandResponse {
    match name {
        commands::META_GET => {
            let meta = MetaInfo {
                edition: zk_composed::EDITION.to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                packages: zk_composed::PACKAGES
                    .iter()
                    .map(|package| (*package).to_owned())
                    .collect(),
                agent_config_error: state.agent_config_error(),
            };
            CommandResponse::ok(serde_json::to_value(&meta).unwrap_or(Value::Null))
        }
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
            // The UI never receives stored secrets (masked by settings.get); an
            // empty field means "keep the existing secret" (provider + embedding).
            let existing = state.runtime.settings().load().ok();
            keep_existing_secrets(&mut settings, existing.as_ref());
            if let Err(error) = validate_settings(&settings) {
                return CommandResponse::err(error);
            }
            if let Err(error) = apply_embedding_settings(&settings) {
                return CommandResponse::err(error);
            }
            // Apply agent + tool settings first: validation/runtime failures
            // must reach the UI instead of persisting settings that never take
            // effect.
            if let Err(error) = apply_agent_settings(&settings) {
                return CommandResponse::err(error);
            }
            if let Err(error) = apply_schedule_settings(&settings) {
                return CommandResponse::err(error);
            }
            // Publish only explicitly saved non-default sections (env fallback
            // stays authoritative for untouched/default sections).
            if let Err(error) =
                publish_runtime_settings(&runtime_snapshot_for(&settings, true, true))
            {
                return CommandResponse::err(error);
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
        commands::WORKSPACE_CHANGES => {
            let Some(workspace_id) = payload.get("workspace_id").and_then(Value::as_str) else {
                return CommandResponse::err("missing workspace_id");
            };
            match state.runtime.sessions().list_workspaces() {
                Ok(workspaces) => match workspaces
                    .iter()
                    .find(|workspace| workspace.id == workspace_id)
                {
                    Some(workspace) => CommandResponse::ok(json!(
                        crate::changes::workspace_changes(&workspace.path)
                    )),
                    None => CommandResponse::err("workspace/not-found"),
                },
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
                        Ok(result) => CommandResponse::ok(
                            serde_json::to_value(&result).unwrap_or(Value::Null),
                        ),
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
        commands::SYSTEM_OPEN_URL => open_url(payload),
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
fn list_workspace_files(root: &str) -> Result<WorkspaceFilesResult, String> {
    const MAX_ENTRIES: usize = 300;
    let mut items: Vec<FileEntry> = Vec::new();
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
            items.push(FileEntry {
                path: rel,
                depth: u32::try_from(depth).unwrap_or(u32::MAX),
                kind: if is_dir {
                    FileKind::Dir
                } else {
                    FileKind::File
                },
                badge: String::new(),
            });
            if is_dir {
                stack.push((path, depth.saturating_add(1)));
            }
        }
    }
    items.sort_unstable_by_key(|entry| entry.path.clone());
    Ok(WorkspaceFilesResult { items })
}

/// Validate an external URL for `system.open_url`.
///
/// Only absolute http(s) URLs without credentials or fragments are accepted;
/// the length cap matches the IPC boundary. Pure so tests never launch a browser.
fn validate_open_url(raw: &str) -> Result<String, String> {
    if raw.is_empty() || raw.len() > 2048 {
        return Err("open-url/invalid-url".to_owned());
    }
    let url = tauri::Url::parse(raw).map_err(|_| "open-url/invalid-url".to_owned())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("open-url/invalid-scheme".to_owned());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("open-url/credentials-not-allowed".to_owned());
    }
    if url.fragment().is_some() {
        return Err("open-url/fragment-not-allowed".to_owned());
    }
    Ok(url.to_string())
}

#[cfg(any(test, target_os = "windows"))]
fn windows_open_url_invocation(url: &str) -> (&'static str, Vec<String>) {
    // ShellExecute equivalent without a shell: URL stays a single argv element.
    (
        "rundll32",
        vec!["url.dll,FileProtocolHandler".to_owned(), url.to_owned()],
    )
}

#[cfg(target_os = "windows")]
fn open_url_invocation(url: &str) -> Result<(&'static str, Vec<String>), String> {
    Ok(windows_open_url_invocation(url))
}

#[cfg(target_os = "macos")]
fn open_url_invocation(url: &str) -> Result<(&'static str, Vec<String>), String> {
    Ok(("open", vec![url.to_owned()]))
}

#[cfg(target_os = "linux")]
fn open_url_invocation(url: &str) -> Result<(&'static str, Vec<String>), String> {
    Ok(("xdg-open", vec![url.to_owned()]))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn open_url_invocation(_url: &str) -> Result<(&'static str, Vec<String>), String> {
    Err("open-url/unsupported-platform".to_owned())
}

/// Spawn the opener and reap it on a detached thread (no zombies, no IPC wait).
fn spawn_and_reap(mut command: std::process::Command) -> Result<(), String> {
    let mut child = command
        .spawn()
        .map_err(|error| format!("open-url/launch-failed: {error}"))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn open_platform_browser(url: &str) -> Result<(), String> {
    let (program, args) = open_url_invocation(url)?;
    let mut command = std::process::Command::new(program);
    command
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    spawn_and_reap(command)
}

/// `system.open_url`: validate, then spawn the platform opener without waiting.
fn open_url(payload: &Value) -> CommandResponse {
    let Some(raw) = payload.get("url").and_then(Value::as_str) else {
        return CommandResponse::err("missing url");
    };
    let url = match validate_open_url(raw) {
        Ok(url) => url,
        Err(error) => return CommandResponse::err(error),
    };
    match open_platform_browser(&url) {
        Ok(()) => CommandResponse::ok(json!({ "opened": true })),
        Err(error) => CommandResponse::err(error),
    }
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
    match app_request_builder(&client, &url, &token, &request).send() {
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

/// Build the native app-request call with the shared client identity headers.
fn app_request_builder(
    client: &reqwest::blocking::Client,
    url: &str,
    token: &str,
    request: &Value,
) -> reqwest::blocking::RequestBuilder {
    crate::native_client::apply_headers(client.post(url).bearer_auth(token).json(request))
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
    if let Some(embedding) = value.get_mut("embedding").and_then(Value::as_object_mut) {
        embedding.insert("api_key".to_owned(), Value::String(String::new()));
    }
    value
}

/// Apply the personal-edition embedding provider settings (C7: enterprise never
/// configures a local embedding provider; the generated glue is a no-op there).
pub(crate) fn apply_embedding_settings(settings: &AppSettings) -> Result<(), String> {
    settings
        .embedding
        .validate()
        .map_err(|error| format!("embedding settings: {error}"))?;
    if zk_composed::EDITION == "personal" {
        zk_composed::configure_embedding_from_settings(&settings.embedding);
    }
    Ok(())
}

/// Keep stored write-only secrets when the UI sends a masked (empty) value.
fn keep_existing_secrets(settings: &mut AppSettings, existing: Option<&AppSettings>) {
    let Some(existing) = existing else {
        return;
    };
    if settings.provider.api_key.trim().is_empty() {
        settings.provider.api_key = existing.provider.api_key.clone();
    }
    if settings.embedding.api_key.trim().is_empty() {
        settings.embedding.api_key = existing.embedding.api_key.clone();
    }
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
    if let Err(error) = settings.agent.schedule.validate() {
        return Err(error.to_string());
    }
    if let Err(error) = settings.embedding.validate() {
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
        usage: None,
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

    let result = ChatSendResult {
        stream_id,
        conversation_id,
        workspace_id,
    };
    CommandResponse::ok(serde_json::to_value(&result).unwrap_or(Value::Null))
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
            let status = if job.cancelled.load(Ordering::SeqCst) {
                "cancelled"
            } else {
                "done"
            };
            let message = assistant_message(&job.conversation_id, reply.text, reply.usage, status);
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
                usage: None,
            };
            let _ = state.runtime.sessions().append_message(&message);
        }
    }
    if let Ok(mut streams) = state.streams.lock() {
        streams.remove(&job.stream_id);
    }
}

/// Build the persisted assistant message for one finished turn.
///
/// Token usage travels with the reply (round-4): persisted so it survives
/// restarts; absent usage stays `None` (SQL NULL).
fn assistant_message(
    conversation_id: &str,
    text: String,
    usage: Option<zk_contract_agent::AgentUsage>,
    status: &str,
) -> Message {
    Message {
        id: uuid::Uuid::new_v4().to_string(),
        conversation_id: conversation_id.to_owned(),
        role: "assistant".to_owned(),
        content: text,
        created_at_ms: now_ms(),
        status: status.to_owned(),
        usage,
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
    fn published_runtime_snapshot_never_contains_provider_secrets() {
        let mut settings = zk_contract_config::AppSettings::default();
        settings.provider.api_key = "sk-desktop-super-secret".to_owned();
        settings.provider.base_url = "https://upstream.example".to_owned();
        settings.agent.max_steps = 7;
        settings.tools.tool_timeout_ms = 5_000;
        super::publish_runtime_settings(&super::runtime_snapshot_for(&settings, true, true))
            .unwrap();

        let published = zk_contract_config::runtime_settings().unwrap().unwrap();
        assert_eq!(published.agent.as_ref().unwrap().max_steps, 7);
        assert_eq!(published.tools.unwrap().tool_timeout_ms, 5_000);
        let json = serde_json::to_string(&published).unwrap();
        assert!(!json.contains("api_key"), "snapshot leaked api_key: {json}");
        assert!(
            !json.contains("sk-desktop-super-secret"),
            "snapshot leaked provider key value: {json}"
        );
        assert!(
            !json.contains("upstream.example"),
            "snapshot leaked base_url: {json}"
        );
        zk_contract_config::clear_runtime_settings().unwrap();
    }

    #[test]
    fn runtime_snapshot_publishes_only_non_default_sections() {
        let defaults = zk_contract_config::AppSettings::default();
        let empty = super::runtime_snapshot_for(&defaults, true, true);
        assert!(empty.agent.is_none(), "default agent section not published");
        assert!(empty.tools.is_none(), "default tools section not published");

        let mut tools_only = zk_contract_config::AppSettings::default();
        tools_only.tools.tool_timeout_ms = 5_000;
        let snapshot = super::runtime_snapshot_for(&tools_only, true, true);
        assert!(
            snapshot.agent.is_none(),
            "tools-only save must not publish agent defaults"
        );
        assert_eq!(snapshot.tools.unwrap().tool_timeout_ms, 5_000);

        let mut agent_only = zk_contract_config::AppSettings::default();
        agent_only.agent.max_steps = 12;
        let snapshot = super::runtime_snapshot_for(&agent_only, true, true);
        assert_eq!(snapshot.agent.unwrap().max_steps, 12);
        assert!(
            snapshot.tools.is_none(),
            "agent-only save must not publish tool defaults"
        );

        // Boot must respect file existence for both sections.
        let nothing = super::runtime_snapshot_for(&agent_only, false, false);
        assert!(nothing.agent.is_none() && nothing.tools.is_none());
    }

    #[test]
    fn invalid_tool_settings_are_rejected_by_validate_settings() {
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
    fn assistant_message_persists_usage_and_is_optional() {
        let usage = zk_contract_agent::AgentUsage {
            input_tokens: 9,
            output_tokens: 4,
            total_tokens: 13,
            cached_tokens: 0,
            reasoning_tokens: 1,
            cost_micros: None,
        };
        let with_usage = super::assistant_message("c1", "answer".to_owned(), Some(usage), "done");
        assert_eq!(with_usage.usage, Some(usage));
        assert_eq!(with_usage.status, "done");
        assert_eq!(with_usage.role, "assistant");
        let without = super::assistant_message("c1", "answer".to_owned(), None, "cancelled");
        assert!(without.usage.is_none());
        assert_eq!(without.status, "cancelled");
    }

    #[test]
    fn embedding_settings_apply_mask_and_validation() {
        let settings = zk_contract_config::AppSettings {
            embedding: zk_contract_config::EmbeddingSettings {
                base_url: "https://api.example.invalid/v1".to_owned(),
                api_key: "sk-embed-secret".to_owned(),
                model: "text-embedding-3-small".to_owned(),
                dims: Some(8),
                dir: String::new(),
            },
            ..zk_contract_config::AppSettings::default()
        };
        super::apply_embedding_settings(&settings).unwrap();

        // `settings.get` must mask the write-only embedding key (like provider).
        let masked = super::masked_settings(&settings);
        assert_eq!(masked["provider"]["api_key"], "");
        assert_eq!(masked["embedding"]["api_key"], "");
        assert_eq!(masked["embedding"]["model"], "text-embedding-3-small");

        // A masked (empty) incoming key keeps the stored secret on save.
        let mut incoming = settings.clone();
        incoming.embedding.api_key = String::new();
        super::keep_existing_secrets(&mut incoming, Some(&settings));
        assert_eq!(incoming.embedding.api_key, "sk-embed-secret");
        assert_eq!(
            super::masked_settings(&incoming)["embedding"]["api_key"],
            ""
        );

        // Invalid dims/scheme are rejected before persisting/applying.
        let mut invalid = settings.clone();
        invalid.embedding.dims = Some(0);
        assert!(super::apply_embedding_settings(&invalid).is_err());
        assert!(super::validate_settings(&invalid).is_err());
        invalid.embedding.dims = Some(8);
        invalid.embedding.base_url = "ftp://example.invalid".to_owned();
        assert!(super::apply_embedding_settings(&invalid).is_err());
        assert!(super::validate_settings(&invalid).is_err());
    }

    #[test]
    fn schedule_settings_apply_path_and_env_fallback() {
        let _env_guard = crate::automation::ENV_TEST_LOCK.lock().unwrap();
        std::env::set_var(crate::automation::ENABLE_ENV, "1");
        crate::automation::clear_settings_override();
        // Boot with NO settings file: env fallback applies.
        super::apply_persisted_schedule(&zk_contract_config::AppSettings::default(), false)
            .unwrap();
        let config = crate::automation::effective_config();
        assert!(config.enabled, "fresh install uses the env fallback");
        assert_eq!(config.scan_seconds, crate::automation::DEFAULT_SCAN_SECONDS);

        // Boot with a settings file whose schedule is the default: explicit
        // save wins, so auto-run is disabled even though env=1 (round-9 P1-1).
        super::apply_persisted_schedule(&zk_contract_config::AppSettings::default(), true).unwrap();
        let config = crate::automation::effective_config();
        assert!(
            !config.enabled,
            "explicit default schedule disables env auto-run"
        );
        assert_eq!(config.scan_seconds, crate::automation::DEFAULT_SCAN_SECONDS);

        // settings.set with explicitly saved enabled/30 wins.
        let mut custom = zk_contract_config::AppSettings::default();
        custom.agent.schedule = zk_contract_config::ScheduleSettings {
            enabled: true,
            scan_seconds: 30,
        };
        super::apply_schedule_settings(&custom).unwrap();
        let config = crate::automation::effective_config();
        assert!(config.enabled);
        assert_eq!(config.scan_seconds, 30);

        // settings.set with explicitly saved disabled/45 is honoured (env still 1).
        let mut disabled = zk_contract_config::AppSettings::default();
        disabled.agent.schedule = zk_contract_config::ScheduleSettings {
            enabled: false,
            scan_seconds: 45,
        };
        super::apply_schedule_settings(&disabled).unwrap();
        let config = crate::automation::effective_config();
        assert!(!config.enabled);
        assert_eq!(config.scan_seconds, 45);

        // Out-of-range values are rejected before persisting.
        let mut invalid = custom.clone();
        invalid.agent.schedule.scan_seconds = 4;
        assert!(super::apply_schedule_settings(&invalid).is_err());
        assert!(super::validate_settings(&invalid).is_err());

        // Fresh install (no file) + no env -> disabled/default.
        std::env::remove_var(crate::automation::ENABLE_ENV);
        super::apply_persisted_schedule(&zk_contract_config::AppSettings::default(), false)
            .unwrap();
        let config = crate::automation::effective_config();
        assert!(!config.enabled);
        assert_eq!(config.scan_seconds, crate::automation::DEFAULT_SCAN_SECONDS);
        crate::automation::clear_settings_override();
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

        // Tools-only save (default agent section) must NOT pin defaults in the
        // legacy registry: it clears it so ZEOKEEL_AGENT_* env stays authoritative.
        let mut tools_only = zk_contract_config::AppSettings::default();
        tools_only.tools.tool_timeout_ms = 5_000;
        super::apply_agent_settings(&tools_only).unwrap();
        assert!(
            zk_contract_agent::default_agent_settings().is_none(),
            "tools-only save must clear the agent registry"
        );
        // settings.set also publishes the per-section snapshot: agent None.
        let snapshot = super::runtime_snapshot_for(&tools_only, true, true);
        super::publish_runtime_settings(&snapshot).unwrap();
        let published = zk_contract_config::runtime_settings().unwrap().unwrap();
        assert!(
            published.agent.is_none(),
            "tools-only save must not publish an agent snapshot"
        );
        assert_eq!(published.tools.unwrap().tool_timeout_ms, 5_000);
        zk_contract_config::clear_runtime_settings().unwrap();

        // Save-custom then reset-to-default also clears the registry (env again).
        let mut custom = zk_contract_config::AppSettings::default();
        custom.agent.max_steps = 9;
        super::apply_agent_settings(&custom).unwrap();
        assert_eq!(
            zk_contract_agent::default_agent_settings()
                .unwrap()
                .max_steps,
            9
        );
        super::apply_agent_settings(&zk_contract_config::AppSettings::default()).unwrap();
        assert!(zk_contract_agent::default_agent_settings().is_none());
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
    #[test]
    fn host_result_dtos_serialize_the_generated_contract_shapes() {
        let meta = zk_contract_ipc::results::MetaInfo {
            edition: "personal".to_owned(),
            version: "0.1.0".to_owned(),
            packages: vec!["zeokeel/desktop".to_owned()],
            agent_config_error: None,
        };
        let meta_value = serde_json::to_value(&meta).unwrap();
        assert_eq!(meta_value["edition"], "personal");
        assert_eq!(meta_value["version"], "0.1.0");
        assert!(meta_value["packages"].is_array());
        assert!(meta_value.get("agent_config_error").is_some());

        let chat = zk_contract_ipc::results::ChatSendResult {
            stream_id: "stream-1".to_owned(),
            conversation_id: "conversation-1".to_owned(),
            workspace_id: "workspace-1".to_owned(),
        };
        let chat_value = serde_json::to_value(&chat).unwrap();
        assert_eq!(chat_value["stream_id"], "stream-1");
        assert_eq!(chat_value["conversation_id"], "conversation-1");
        assert_eq!(chat_value["workspace_id"], "workspace-1");

        let root = std::env::temp_dir().join(format!("zk-ipc-dto-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("main.rs"), "fn main() {}\n").unwrap();
        let files = super::list_workspace_files(root.to_str().unwrap()).unwrap();
        let files_value = serde_json::to_value(&files).unwrap();
        assert!(files_value["items"].is_array());
        assert_eq!(files_value["items"][0]["path"], "main.rs");
        assert_eq!(files_value["items"][0]["kind"], "file");
        assert!(files_value["items"][0]["badge"].is_string());
        std::fs::remove_dir_all(&root).unwrap();
    }
    #[test]
    fn open_url_invocation_keeps_url_as_one_arg_without_a_shell() {
        let url = "https://host.example/portal?a=1&calc";
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let (program, args) = super::open_url_invocation(url).expect("unix opener");
            assert!(program == "xdg-open" || program == "open", "{program}");
            assert_eq!(args, vec![url.to_owned()]);
        }
        let (program, args) = super::windows_open_url_invocation(url);
        assert_eq!(program, "rundll32");
        assert_eq!(
            args,
            vec!["url.dll,FileProtocolHandler".to_owned(), url.to_owned()]
        );
        assert!(!args.iter().any(|arg| arg == "cmd" || arg == "/C"));
    }

    #[cfg(unix)]
    #[test]
    fn open_url_child_is_spawned_and_reaped_without_waiting() {
        let mut command = std::process::Command::new("true");
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        super::spawn_and_reap(command).expect("spawn/reap helper");
    }

    #[cfg(unix)]
    #[test]
    fn open_url_spawn_failure_is_returned() {
        let command = std::process::Command::new("/definitely/not/a/real/zk-opener");
        let error = super::spawn_and_reap(command).expect_err("missing program must fail");
        assert!(error.starts_with("open-url/launch-failed:"), "{error}");
    }

    #[test]
    fn system_open_url_validates_scheme_length_and_credentials() {
        assert_eq!(
            super::validate_open_url("https://example.com/portal?x=1"),
            Ok("https://example.com/portal?x=1".to_owned())
        );
        assert_eq!(
            super::validate_open_url("http://127.0.0.1:8787/portal"),
            Ok("http://127.0.0.1:8787/portal".to_owned())
        );
        for bad in [
            "",
            "ftp://example.com/file",
            "javascript:alert(1)",
            "file:///etc/passwd",
            "https://user:pass@example.com/",
            "https://example.com/#fragment",
        ] {
            assert!(
                super::validate_open_url(bad).is_err(),
                "must reject {bad:?}"
            );
        }
        assert!(
            super::validate_open_url(&format!("https://example.com/{}", "a".repeat(2049))).is_err()
        );
        let response = super::open_url(&serde_json::json!({"url": "javascript:alert(1)"}));
        assert!(!response.ok);
        assert_eq!(response.error.as_deref(), Some("open-url/invalid-scheme"));
        let missing = super::open_url(&serde_json::json!({}));
        assert!(!missing.ok);
        assert_eq!(missing.error.as_deref(), Some("missing url"));
    }
    #[test]
    fn app_request_builder_attaches_native_identity_and_request_id() {
        let client = reqwest::blocking::Client::new();
        let request = serde_json::json!({
            "abi": "zeokeel-app/1",
            "app_id": "demo",
            "method": "GET",
        });
        let built = super::app_request_builder(
            &client,
            "http://127.0.0.1:1/api/client/v1/apps/demo/request",
            "token",
            &request,
        )
        .build()
        .unwrap();
        let headers = built.headers();
        let client_header = headers
            .get("x-zeokeel-client")
            .and_then(|value| value.to_str().ok())
            .expect("client header");
        assert_eq!(client_header.split(';').count(), 4);
        assert!(
            headers.get("x-request-id").is_some(),
            "request id header missing"
        );
    }
}
