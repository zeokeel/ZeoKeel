#![forbid(unsafe_code)]
//! Shared command dispatch used by both the Tauri shell and the HTTP serve carrier.

use std::collections::HashMap;
use std::io::{Read as _, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use base64::Engine as _;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use zk_contract_agent::ApprovalDecision;
use zk_contract_agent::{AgentHookSink, HistoryMessage, HistoryRole};
use zk_contract_config::{AppSettings, ProviderKind, ToolSettings, WebSearchSettings};
use zk_contract_ipc::results::{ChatSendResult, MetaInfo};
use zk_contract_ipc::{commands, payloads::ApprovalDecidePayload, CommandResponse, UiEvent};
use zk_contract_session::{FileEntry, FileKind, Message, WorkspaceFilesResult};
use zk_contract_tool::TurnContext;

use crate::approval::ApprovalBroker;
use crate::events::{now_ms, BridgeSink, EVENT_BUFFER};
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
    streams: Mutex<HashMap<String, Arc<AtomicBool>>>,
    turns: Mutex<HashMap<String, mpsc::Sender<TurnJob>>>,
    approvals: Arc<ApprovalBroker>,
    questions: Arc<QuestionBroker>,
    agent_config_error: Mutex<Option<String>>,
    /// Background MCP connector OAuth flows (state only; never tokens).
    oauth_flows: Mutex<HashMap<String, OAuthFlowStatus>>,
    /// Two-phase MCP connection probes (`mcp.test` / `mcp.test.status`).
    mcp_probes: Arc<crate::mcp::McpProbeBroker>,
    /// Host-cached enterprise gateway model ids (login/refresh updates; logout clears).
    gateway_models: Mutex<Vec<String>>,
    /// Conversations with an actively running agent turn (`/compact` busy guard).
    running_conversations: Mutex<std::collections::HashSet<String>>,
}

/// One background connector OAuth flow (`docs/07` §6): `pending|authorized|failed`.
///
/// Deliberately holds no token material — tokens go straight to the keychain via
/// the existing `oauth.save` store, and errors carry status codes only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OAuthFlowStatus {
    /// `pending` while the browser/loopback flow is running.
    pub state: String,
    /// Stable error code when `state == "failed"` (no token material).
    pub error: Option<String>,
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
            streams: Mutex::new(HashMap::new()),
            turns: Mutex::new(HashMap::new()),
            approvals,
            questions,
            agent_config_error: Mutex::new(None),
            oauth_flows: Mutex::new(HashMap::new()),
            mcp_probes: crate::mcp::McpProbeBroker::new(),
            gateway_models: Mutex::new(Vec::new()),
            running_conversations: Mutex::new(std::collections::HashSet::new()),
        })
    }

    /// Runtime accessor (tests).
    pub fn runtime(&self) -> &Arc<zk_composed::Runtime> {
        &self.runtime
    }

    /// Whether one conversation currently runs an agent turn.
    pub fn is_conversation_running(&self, conversation_id: &str) -> bool {
        self.running_conversations
            .lock()
            .map(|running| running.contains(conversation_id))
            .unwrap_or(false)
    }

    fn mark_conversation_running(&self, conversation_id: &str) {
        if let Ok(mut running) = self.running_conversations.lock() {
            running.insert(conversation_id.to_owned());
        }
    }

    fn unmark_conversation_running(&self, conversation_id: &str) {
        if let Ok(mut running) = self.running_conversations.lock() {
            running.remove(conversation_id);
        }
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
    include_web_search: bool,
) -> zk_contract_config::RuntimeSettings {
    zk_contract_config::RuntimeSettings {
        agent: include_agent
            .then(|| settings.agent.clone())
            .filter(|agent| *agent != zk_contract_config::AgentSettings::default()),
        tools: include_tools
            .then_some(settings.tools)
            .filter(|tools| *tools != ToolSettings::default()),
        web_search: include_web_search
            .then(|| settings.web_search.clone())
            .filter(|web_search| *web_search != WebSearchSettings::default()),
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

/// Boot path for personal-edition local capability providers (embedding + media).
///
/// Personal edition: load the saved settings and apply them (propagating the
/// first failed config so `main` records it in `meta.get.agent_config_error`).
/// Enterprise: never hold/persist local providers (C7) — ignore them and clear
/// any stale personal-edition values from the settings file.
pub(crate) fn configure_persisted_capabilities(
    runtime: &zk_composed::Runtime,
) -> Result<(), String> {
    let mut settings = runtime
        .settings()
        .load()
        .map_err(|error| format!("settings load: {error}"))?;
    if zk_composed::EDITION != "personal" {
        let embedding_stale =
            settings.embedding != zk_contract_config::EmbeddingSettings::default();
        let media_stale = settings.media != zk_contract_config::MediaSettings::default();
        if embedding_stale || media_stale {
            settings.embedding = zk_contract_config::EmbeddingSettings::default();
            settings.media = zk_contract_config::MediaSettings::default();
            runtime
                .settings()
                .save(&settings)
                .map_err(|error| format!("clear persisted capability providers: {error}"))?;
            eprintln!(
                "zk-desktop: cleared persisted embedding/media providers (enterprise is gateway-only, C7)"
            );
        }
        return Ok(());
    }
    apply_embedding_settings(&settings)?;
    apply_media_settings(&settings)
}

/// Boot path: load persisted settings and publish the agent section only when
/// the settings file was explicitly saved with non-default agent settings.
///
/// On a fresh install nothing is published, so `zk-agent-rig` keeps its
/// `ZEOKEEL_AGENT_*` environment fallback (dev/e2e flows).
pub(crate) fn apply_persisted_settings(
    runtime: &zk_composed::Runtime,
) -> Result<Option<ToolSettings>, String> {
    let mut settings = runtime
        .settings()
        .load()
        .map_err(|error| format!("settings load: {error}"))?;
    let file_exists = settings_file_exists();
    let mut dirty = false;
    // Boot normalization (config spec R4): canonicalize the deprecated
    // `auto_approve_read_only` field into `tools.approval_policy`. A fresh
    // install is never persisted so the env/default fallback stays active.
    if file_exists {
        let before = settings.clone();
        settings.normalize_approval_policy();
        dirty |= settings != before;
    }
    // Enterprise must never hold/persist a local embedding provider (C7):
    // ignore and clear any stale personal-edition values.
    if zk_composed::EDITION != "personal"
        && settings.embedding != zk_contract_config::EmbeddingSettings::default()
    {
        settings.embedding = zk_contract_config::EmbeddingSettings::default();
        dirty = true;
        eprintln!(
            "zk-desktop: cleared persisted embedding provider (enterprise is gateway-only, C7)"
        );
    }
    // Enterprise must never hold/persist a local MCP section (catalog-only, C6):
    // ignore and clear any stale personal-edition values.
    if zk_composed::EDITION != "personal"
        && settings.mcp != zk_contract_config::McpSettings::default()
    {
        settings.mcp = zk_contract_config::McpSettings::default();
        dirty = true;
        eprintln!(
            "zk-desktop: cleared persisted local MCP servers (enterprise is catalog-only, C6)"
        );
    }
    if dirty {
        match runtime.settings().save(&settings) {
            Ok(()) => {
                eprintln!("zk-desktop: persisted normalized settings (approval policy/edition)")
            }
            Err(error) => eprintln!("zk-desktop: failed to persist normalized settings: {error}"),
        }
    }
    // A settings file (even one carrying the default schedule) is explicit:
    // it overrides env. Only a fresh install (no file) uses the env fallback.
    apply_persisted_schedule(&settings, file_exists)?;
    if let Err(error) = crate::mcp::apply_persisted_mcp(&settings, file_exists) {
        eprintln!("zk-desktop: failed to apply persisted MCP settings: {error}");
    }
    if should_publish_agent_settings(&settings, file_exists) {
        apply_agent_settings(&settings)?;
    }
    let snapshot = runtime_snapshot_for(&settings, file_exists, file_exists, file_exists);
    if snapshot.agent.is_some() || snapshot.tools.is_some() || snapshot.web_search.is_some() {
        publish_runtime_settings(&snapshot)?;
    }
    Ok(snapshot.tools)
}

fn apply_connector_catalog(catalog_json: &str) -> Result<usize, String> {
    let count = zk_connector_mcp::update_from_catalog(catalog_json)?;
    restore_connector_oauth_tokens()?;
    Ok(count)
}

/// Map connector catalog/session failures to the closed IPC error set
/// (config spec C-3): invalid catalogs are `connectors/catalog-invalid`, any
/// other apply/session failure keeps its phase (`apply`/`reload`/`clear`).
fn connectors_error(phase: &str, error: &str) -> String {
    let invalid = error.starts_with("MCP_CONFIG")
        || error.starts_with("MCP_VALIDATION")
        || error.starts_with("MCP_INSECURE");
    if invalid {
        return format!("connectors/catalog-invalid: {error}");
    }
    match phase {
        "reload" => format!("connectors/reload-failed: {error}"),
        "clear" => format!("connectors/clear-failed: {error}"),
        _ => format!("connectors/apply-failed: {error}"),
    }
}

/// Re-install client-held OAuth tokens into the MCP transport after a catalog sync.
///
/// Tokens live in the OS keychain (`oauth.save`); the transport keeps them in
/// memory and must be told again after every force-update/login (it is cleared on
/// logout by `clear_catalog_config`). Failures are per-server and non-fatal: the
/// affected connector then fails honestly with `MCP_OAUTH_REQUIRED` on use.
fn restore_connector_oauth_tokens() -> Result<(), String> {
    let server_ids = zk_connector_mcp::configured_server_ids()?;
    for server_id in server_ids {
        let Ok(Some(raw)) = oauth_stored_raw(&server_id) else {
            continue;
        };
        let Ok(tokens) = crate::mcp_oauth::OAuthTokens::from_stored_json(&raw) else {
            continue;
        };
        let _ = zk_connector_mcp::set_oauth_token(&server_id, Some(tokens.access_token));
    }
    Ok(())
}

/// Fetch the enterprise connector catalog and install it as MCP client config.
///
/// Called after successful login/bootstrap (`auth.configure`); a failure is
/// reported to the caller but must not break login (tools then stay honestly
/// unavailable until the next successful sync).
fn sync_connector_catalog(server_url: &str, access_token: &str) -> Result<usize, String> {
    let base = server_url.trim_end_matches('/');
    if base.is_empty() || access_token.trim().is_empty() {
        return Err("connectors/catalog: missing server_url/access_token".to_owned());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|error| format!("connectors/catalog: client: {error}"))?;
    let request =
        crate::native_client::apply_headers(client.get(format!("{base}/api/client/v1/connectors")))
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {access_token}"),
            );
    let response = request
        .send()
        .map_err(|error| format!("connectors/catalog: request: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "connectors/catalog: HTTP {}",
            response.status().as_u16()
        ));
    }
    let body = response
        .text()
        .map_err(|error| format!("connectors/catalog: body: {error}"))?;
    apply_connector_catalog(&body)
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
            Ok(settings) => match state.runtime.settings().revision() {
                Ok(revision) => CommandResponse::ok(json!({
                    "settings": masked_settings(&settings),
                    "revision": revision,
                })),
                Err(error) => CommandResponse::err(error.to_string()),
            },
            Err(error) => CommandResponse::err(error.to_string()),
        },
        commands::SETTINGS_SET => {
            let Some(value) = payload.get("settings") else {
                return CommandResponse::err("missing settings");
            };
            // Optional optimistic-concurrency fence (absent = legacy unfenced save).
            let expected_revision = match payload.get("expected_revision") {
                None | Some(Value::Null) => None,
                Some(value) => match value.as_u64() {
                    Some(revision) => Some(revision),
                    None => {
                        return CommandResponse::err(
                            "invalid expected_revision: must be an unsigned integer",
                        )
                    }
                },
            };
            let mut settings: AppSettings = match serde_json::from_value(value.clone()) {
                Ok(settings) => settings,
                Err(error) => return CommandResponse::err(format!("invalid settings: {error}")),
            };
            // The UI never receives stored secrets (masked by settings.get); an
            // empty field means "keep the existing secret" (provider + embedding).
            let existing = state.runtime.settings().load().ok();
            // Reject a stale fence BEFORE any runtime side effect: a conflict must
            // not write settings nor re-point the running agent/provider.
            let current_revision = match state.runtime.settings().revision() {
                Ok(revision) => revision,
                Err(error) => return CommandResponse::err(error.to_string()),
            };
            if expected_revision.is_some_and(|expected| expected != current_revision) {
                if let Some(expected) = expected_revision {
                    return CommandResponse::err(settings_conflict_error(
                        expected,
                        current_revision,
                    ));
                }
            }
            keep_existing_secrets(&mut settings, existing.as_ref());
            // Config spec R4: canonicalize the legacy approval field before
            // validating/publishing/persisting (idempotent).
            settings.normalize_approval_policy();
            if let Err(error) = validate_settings(&settings) {
                return CommandResponse::err(error);
            }
            if let Err(error) = apply_embedding_settings(&settings) {
                return CommandResponse::err(error);
            }
            if let Err(error) = apply_media_settings(&settings) {
                return CommandResponse::err(error);
            }
            // Config-mutex: a running turn holds the shared agent config for its
            // conversation override; settings.apply must queue behind it so it
            // cannot change the model/permission of an in-flight turn.
            let _config_guard = crate::command_surface::lock_config();
            // Apply agent + tool settings first: validation/runtime failures
            // must reach the UI instead of persisting settings that never take
            // effect.
            if let Err(error) = apply_agent_settings(&settings) {
                return CommandResponse::err(error);
            }
            if let Err(error) = apply_schedule_settings(&settings) {
                return CommandResponse::err(error);
            }
            // MCP local settings (spec R4): apply before persisting so a failed
            // install never yields "saved but not effective".
            if let Err(error) = crate::mcp::apply_settings_mcp(&settings) {
                return CommandResponse::err(error);
            }
            // Publish only explicitly saved non-default sections (env fallback
            // stays authoritative for untouched/default sections).
            if let Err(error) =
                publish_runtime_settings(&runtime_snapshot_for(&settings, true, true, true))
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
            match state
                .runtime
                .settings()
                .save_with_revision(&settings, expected_revision)
            {
                Ok(revision) => CommandResponse::ok(json!({
                    "settings": masked_settings(&settings),
                    "revision": revision,
                })),
                Err(zk_contract_config::ConfigError::Conflict(message)) => {
                    CommandResponse::err(format!("settings/revision-conflict: {message}"))
                }
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
        commands::WORKSPACE_FILES => workspace_files(state, payload),
        commands::WORKSPACE_FILE_READ => workspace_file_read(state, payload),
        commands::TERMINAL => terminal_command(state, payload),
        commands::SESSION_UPDATE => session_update(state, payload),
        commands::GOAL_GET => {
            let request: zk_contract_ipc::payloads::GoalGetPayload =
                match serde_json::from_value(payload.clone()) {
                    Ok(request) => request,
                    Err(error) => return CommandResponse::err(format!("goal/invalid: {error}")),
                };
            match crate::command_surface::goal_get(
                &*state.runtime.sessions(),
                &request.workspace_id,
                &request.conversation_id,
            ) {
                Ok(result) => CommandResponse::ok(json!(result)),
                Err(error) => CommandResponse::err(error),
            }
        }
        commands::GOAL_SET => {
            let request: zk_contract_ipc::payloads::GoalSetPayload =
                match serde_json::from_value(payload.clone()) {
                    Ok(request) => request,
                    Err(error) => return CommandResponse::err(format!("goal/invalid: {error}")),
                };
            match crate::command_surface::goal_set(&*state.runtime.sessions(), &request) {
                Ok(result) => CommandResponse::ok(json!(result)),
                Err(error) => CommandResponse::err(error),
            }
        }
        commands::GOAL_UPDATE => {
            let request: zk_contract_ipc::payloads::GoalUpdatePayload =
                match serde_json::from_value(payload.clone()) {
                    Ok(request) => request,
                    Err(error) => return CommandResponse::err(format!("goal/invalid: {error}")),
                };
            match crate::command_surface::goal_update(&*state.runtime.sessions(), &request) {
                Ok(result) => CommandResponse::ok(json!(result)),
                Err(error) => CommandResponse::err(error),
            }
        }
        commands::SESSION_OVERRIDE_GET => {
            let request: zk_contract_ipc::payloads::ConversationOverrideGetPayload =
                match serde_json::from_value(payload.clone()) {
                    Ok(request) => request,
                    Err(error) => return CommandResponse::err(format!("session/invalid: {error}")),
                };
            match crate::command_surface::get_override(
                &*state.runtime.sessions(),
                &request.workspace_id,
                &request.conversation_id,
            ) {
                Ok(result) => CommandResponse::ok(json!(result)),
                Err(error) => CommandResponse::err(error),
            }
        }
        commands::SESSION_OVERRIDE_SET => {
            let request: zk_contract_ipc::payloads::ConversationOverrideSetPayload =
                match serde_json::from_value(payload.clone()) {
                    Ok(request) => request,
                    Err(error) => return CommandResponse::err(format!("session/invalid: {error}")),
                };
            let models = state
                .gateway_models
                .lock()
                .map(|models| models.clone())
                .unwrap_or_default();
            match crate::command_surface::set_override(
                &*state.runtime.sessions(),
                &request.workspace_id,
                &request.conversation_id,
                request.model.as_deref(),
                request.reasoning_effort.as_deref(),
                request.approval_policy.as_deref(),
                &models,
            ) {
                Ok(result) => CommandResponse::ok(json!(result)),
                Err(error) => CommandResponse::err(error),
            }
        }
        commands::SESSION_EXPORT => {
            let request: zk_contract_ipc::payloads::SessionExportPayload =
                match serde_json::from_value(payload.clone()) {
                    Ok(request) => request,
                    Err(error) => return CommandResponse::err(format!("session/invalid: {error}")),
                };
            if request.format != "markdown" {
                return CommandResponse::err("session/invalid: unsupported export format");
            }
            match crate::command_surface::export_markdown_file(
                &*state.runtime.sessions(),
                &request.workspace_id,
                &request.conversation_id,
            ) {
                Ok(result) => CommandResponse::ok(json!(result)),
                Err(error) => CommandResponse::err(error),
            }
        }
        commands::SESSION_COMPACT => {
            let request: zk_contract_ipc::payloads::SessionCompactPayload =
                match serde_json::from_value(payload.clone()) {
                    Ok(request) => request,
                    Err(error) => return CommandResponse::err(format!("compact/invalid: {error}")),
                };
            match crate::compact::compact_conversation(
                state,
                &request.workspace_id,
                &request.conversation_id,
            ) {
                Ok(result) => CommandResponse::ok(json!(result)),
                Err(error) => CommandResponse::err(error),
            }
        }
        commands::SESSION_COMPACTION_GET => {
            let request: zk_contract_ipc::payloads::CompactionGetPayload =
                match serde_json::from_value(payload.clone()) {
                    Ok(request) => request,
                    Err(error) => return CommandResponse::err(format!("compact/invalid: {error}")),
                };
            match crate::compact::compaction_get(
                &*state.runtime().sessions(),
                &request.workspace_id,
                &request.conversation_id,
            ) {
                Ok(result) => CommandResponse::ok(json!(result)),
                Err(error) => CommandResponse::err(error),
            }
        }
        commands::SESSION_COMPACTED_VIEW => {
            let request: zk_contract_ipc::payloads::CompactionGetPayload =
                match serde_json::from_value(payload.clone()) {
                    Ok(request) => request,
                    Err(error) => return CommandResponse::err(format!("compact/invalid: {error}")),
                };
            match crate::compact::compacted_view(
                &*state.runtime().sessions(),
                &request.workspace_id,
                &request.conversation_id,
            ) {
                Ok(result) => CommandResponse::ok(json!(result)),
                Err(error) => CommandResponse::err(error),
            }
        }
        commands::PLAN_GET => {
            let request: zk_contract_ipc::payloads::PlanGetPayload =
                match serde_json::from_value(payload.clone()) {
                    Ok(request) => request,
                    Err(error) => return CommandResponse::err(format!("plan/invalid: {error}")),
                };
            match crate::plan::plan_get(state, &request.workspace_id, &request.conversation_id) {
                Ok(result) => CommandResponse::ok(json!(result)),
                Err(error) => CommandResponse::err(error),
            }
        }
        commands::PLAN_UPDATE => {
            let request: zk_contract_ipc::payloads::PlanUpdatePayload =
                match serde_json::from_value(payload.clone()) {
                    Ok(request) => request,
                    Err(error) => return CommandResponse::err(format!("plan/invalid: {error}")),
                };
            match crate::plan::plan_update(
                state,
                &request.workspace_id,
                &request.conversation_id,
                &request.action,
                request.feedback,
            ) {
                Ok(result) => CommandResponse::ok(json!(result)),
                Err(error) => CommandResponse::err(error),
            }
        }
        commands::FEEDBACK_SUBMIT => {
            let request: zk_contract_ipc::payloads::FeedbackSubmitPayload =
                match serde_json::from_value(payload.clone()) {
                    Ok(request) => request,
                    Err(error) => return CommandResponse::err(format!("session/invalid: {error}")),
                };
            match crate::command_surface::submit_feedback(
                &*state.runtime.sessions(),
                &request.workspace_id,
                &request.conversation_id,
                &request.category,
                request.description.as_deref(),
                zk_composed::EDITION,
            ) {
                Ok(result) => CommandResponse::ok(json!(result)),
                Err(error) => CommandResponse::err(error),
            }
        }
        commands::SESSION_FORK => session_fork(state, payload),
        commands::SESSION_SEARCH => session_search(state, payload),
        commands::WORKSPACE_ATTACHMENT_SAVE => attachment_save(state, payload),
        commands::CONNECTORS_CONFIGURE => {
            let Some(catalog) = payload.get("catalog") else {
                return CommandResponse::err("connectors/catalog-invalid: missing catalog");
            };
            let text = match catalog {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            match apply_connector_catalog(&text) {
                Ok(count) => CommandResponse::ok(
                    serde_json::to_value(zk_contract_ipc::results::ConnectorsConfigureResult {
                        configured: u64::try_from(count).unwrap_or(u64::MAX),
                    })
                    .unwrap_or(Value::Null),
                ),
                Err(error) => CommandResponse::err(connectors_error("configure", &error)),
            }
        }
        commands::CONNECTORS_RELOAD => match zk_connector_mcp::reload_connectors() {
            Ok(()) => CommandResponse::ok(
                serde_json::to_value(zk_contract_ipc::results::ConnectorsReloadResult {
                    reloaded: true,
                })
                .unwrap_or(Value::Null),
            ),
            Err(error) => CommandResponse::err(connectors_error("reload", &error)),
        },
        commands::CONNECTORS_CLEAR => match zk_connector_mcp::clear_catalog_config() {
            Ok(()) => CommandResponse::ok(
                serde_json::to_value(zk_contract_ipc::results::ConnectorsClearResult {
                    cleared: true,
                })
                .unwrap_or(Value::Null),
            ),
            Err(error) => CommandResponse::err(connectors_error("clear", &error)),
        },
        commands::MCP_TEST => {
            let Some(server) = payload.get("server") else {
                return CommandResponse::err("mcp/validation: missing server");
            };
            let server: zk_contract_connector::McpServerConfig =
                match serde_json::from_value(server.clone()) {
                    Ok(server) => server,
                    Err(error) => {
                        return CommandResponse::err(format!(
                            "mcp/validation: invalid server: {error}"
                        ))
                    }
                };
            // Reject an invalid draft before starting a probe (spec §2: mcp/validation;
            // local caps/id charset apply to a prospective local server too).
            let draft = zk_contract_config::McpSettings {
                servers: vec![server.clone()],
            };
            if let Err(error) = draft.validate() {
                return CommandResponse::err(error.to_string());
            }
            match state.mcp_probes.start(server) {
                Ok(test_id) => CommandResponse::ok(
                    serde_json::to_value(zk_contract_ipc::results::McpTestResult { test_id })
                        .unwrap_or(Value::Null),
                ),
                Err(error) => CommandResponse::err(error),
            }
        }
        commands::MCP_TEST_STATUS => {
            let Some(test_id) = payload.get("test_id").and_then(Value::as_str) else {
                return CommandResponse::err("mcp/validation: missing test_id");
            };
            match state.mcp_probes.status(test_id) {
                Some(status) => {
                    CommandResponse::ok(serde_json::to_value(status).unwrap_or(Value::Null))
                }
                None => CommandResponse::err("mcp/unsupported: unknown test_id"),
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
            // Enterprise gateway model ids (bootstrap) are cached for the UI and
            // for `session.override_set` validation (C7: no upstream keys here).
            if let Some(models) = payload.get("models").and_then(Value::as_array) {
                let ids: Vec<String> = models
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect();
                if let Ok(mut cache) = state.gateway_models.lock() {
                    *cache = ids;
                }
            }
            // Shared config feeds the gateway credentials/token refresh paths ...
            zk_composed::configure_llm(base_url, access_token, model);
            // ... while this rebuilds the already-constructed rig model so login and
            // refresh take effect without restarting the app (ADR-064 hot-swap).
            match state
                .runtime
                .configure_llm(base_url, access_token, model, "gateway")
            {
                Ok(()) => {
                    // Connector catalog -> MCP client config (B16 phase-3). A catalog
                    // failure must not block login; tools stay honestly unavailable.
                    match sync_connector_catalog(base_url, access_token) {
                        Ok(count) => CommandResponse::ok(json!({
                            "configured": true,
                            "mcp_servers": count,
                        })),
                        Err(error) => {
                            eprintln!("zk: connector catalog sync failed: {error}");
                            CommandResponse::ok(json!({
                                "configured": true,
                                "mcp_servers": 0,
                                "mcp_sync_error": error,
                            }))
                        }
                    }
                }
                Err(error) => CommandResponse::err(format!("auth/configure: {error}")),
            }
        }
        commands::AUTH_LOGOUT => {
            // Clear the enterprise gateway credentials held by the composed runtime.
            // The local session (tokens/server URL) is owned by the UI layer; here we
            // only make sure no in-flight turn can keep using the old credentials.
            zk_composed::configure_llm("", "", "");
            if let Ok(mut cache) = state.gateway_models.lock() {
                cache.clear();
            }
            // Reset the already-built rig model to the boot placeholder so a
            // logged-out agent cannot keep using the old gateway credentials.
            let _ = state.runtime.configure_llm("", "", "", "gateway");
            if let Ok(streams) = state.streams.lock() {
                for cancelled in streams.values() {
                    cancelled.store(true, Ordering::SeqCst);
                }
            }
            state.runtime.cancel_agent();
            // Drop catalog-provided MCP config/sessions on logout (B16 phase-3).
            let _ = zk_connector_mcp::clear_catalog_config();
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
        commands::OAUTH_AUTHORIZE => oauth_authorize(state, payload),
        commands::OAUTH_STATUS => oauth_status(state, payload),
        commands::OAUTH_REFRESH => oauth_refresh(state, payload),
        commands::CHAT_SEND => chat_send(state, payload),
        commands::CHAT_CANCEL => chat_cancel(state, payload),
        commands::APPROVAL_DECIDE => {
            let decide: ApprovalDecidePayload = match serde_json::from_value(payload.clone()) {
                Ok(decide) => decide,
                Err(error) => {
                    return CommandResponse::err(format!("approval/invalid-decision: {error}"));
                }
            };
            let resolved = state
                .approvals
                .resolve_scoped(&decide.approval_id, decide.decision);
            if resolved && matches!(decide.decision, ApprovalDecision::Cancel) {
                // Single cancel path: the agent runtime observes the inherited
                // cancelled flag and emits TurnCancelled once (no host-side emit).
                state.runtime.cancel_agent();
            }
            if resolved {
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

/// One resolved workspace root from the store (canonical absolute path).
fn workspace_root_of(state: &Arc<AppState>, workspace_id: &str) -> Result<PathBuf, String> {
    let workspaces = state
        .runtime
        .sessions()
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

/// Split a workspace-relative forward-slash path into safe components.
fn relative_parts(path: &str) -> Result<Vec<&str>, String> {
    if path.trim().is_empty() {
        return Ok(Vec::new());
    }
    if path.starts_with('/') || path.starts_with('\\') {
        return Err("workspace/path-escape: absolute path".to_owned());
    }
    let parts: Vec<&str> = path.split('/').collect();
    for part in &parts {
        if part.is_empty() || *part == "." || *part == ".." {
            return Err("workspace/path-escape: unsafe component".to_owned());
        }
        if part.contains('\\') || part.chars().any(char::is_control) {
            return Err("workspace/path-escape: unsafe component".to_owned());
        }
    }
    Ok(parts)
}

/// Bounded, read-only listing under one workspace root (lazy tree, Phase 2b R1).
///
/// `rel` is workspace-relative (`""` = root); `depth` counts levels below `rel`
/// (1..=2); entries are sorted directory-first then by name. `.git`/`.zeokeel`
/// are hidden and symlinks are neither followed nor shown.
fn list_workspace_files(
    root: &std::path::Path,
    rel: &str,
    depth: u32,
    limit: u32,
) -> Result<WorkspaceFilesResult, String> {
    let parts = relative_parts(rel)?;
    let base = if parts.is_empty() {
        root.to_path_buf()
    } else {
        zk_composed::zk_kit_tools::safe_resolve_under(root, &parts).map_err(
            |error| match error {
                zk_contract_tool::ToolError::NotFound(_) => "workspace/path-not-found".to_owned(),
                zk_contract_tool::ToolError::Denied(_)
                | zk_contract_tool::ToolError::InvalidArgs(_) => "workspace/path-escape".to_owned(),
                other => format!("workspace/path-unavailable: {other}"),
            },
        )?
    };
    if !base.is_dir() {
        return Err("workspace/not-a-directory".to_owned());
    }
    let limit = usize::try_from(limit).unwrap_or(usize::MAX);
    let mut items: Vec<FileEntry> = Vec::new();
    let mut stack = vec![(base.clone(), 0_u32)];
    while let Some((dir, depth_from_base)) = stack.pop() {
        if depth_from_base >= depth || items.len() >= limit {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut children: Vec<_> = entries.filter_map(Result::ok).collect();
        children.sort_by_key(|entry| entry.file_name());
        for entry in children {
            if items.len() >= limit {
                break;
            }
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == ".git" || name == ".zeokeel" {
                continue;
            }
            let path = entry.path();
            let rel_path = path
                .strip_prefix(root)
                .map(|value| value.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            if rel_path.is_empty() {
                continue;
            }
            let is_dir = file_type.is_dir();
            items.push(FileEntry {
                path: rel_path,
                depth: depth_from_base,
                kind: if is_dir {
                    FileKind::Dir
                } else {
                    FileKind::File
                },
                badge: String::new(),
            });
            if is_dir {
                stack.push((path, depth_from_base.saturating_add(1)));
            }
        }
    }
    // Directory-first, then name, stable across the whole page.
    items.sort_unstable_by(|left, right| {
        let left_dir = left.kind == FileKind::Dir;
        let right_dir = right.kind == FileKind::Dir;
        right_dir
            .cmp(&left_dir)
            .then_with(|| left.path.cmp(&right.path))
    });
    Ok(WorkspaceFilesResult { items })
}

/// `workspace.files`: lazy tree listing (no-argument payload = legacy behaviour).
fn workspace_files(state: &Arc<AppState>, payload: &Value) -> CommandResponse {
    let request: zk_contract_ipc::payloads::WorkspaceFilesPayload =
        match serde_json::from_value(payload.clone()) {
            Ok(request) => request,
            Err(error) => return CommandResponse::err(format!("workspace/invalid: {error}")),
        };
    let root = match workspace_root_of(state, &request.workspace_id) {
        Ok(root) => root,
        Err(error) => return CommandResponse::err(error),
    };
    let depth = request.depth.unwrap_or(2).clamp(1, 2);
    let limit = request.limit.unwrap_or(300).clamp(1, 500);
    match list_workspace_files(&root, &request.path, depth, limit) {
        Ok(result) => CommandResponse::ok(json!(result)),
        Err(error) => CommandResponse::err(error),
    }
}

/// Text preview byte cap (R2: >256 KiB is truncated, not rejected).
const FILE_TEXT_MAX_BYTES: u64 = 256 * 1024;
/// Image preview byte cap (R2: >8 MiB degrades honestly).
const FILE_IMAGE_MAX_BYTES: u64 = 8 * 1024 * 1024;
/// Terminal per-command timeout cap (R3: ≤10 s).
const TERMINAL_MAX_TIMEOUT_MS: u64 = 10_000;
/// Terminal command line cap (R3: input ≤16 KiB).
const TERMINAL_MAX_COMMAND_BYTES: usize = 16 * 1024;
/// Terminal per-read output cap (R3: ≤64 KiB, marked truncated).
const TERMINAL_MAX_OUTPUT_BYTES: usize = 64 * 1024;

fn file_extension(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

fn is_image_extension(extension: &str) -> bool {
    matches!(extension, "png" | "jpg" | "jpeg" | "webp" | "gif")
}

fn is_text_extension(extension: &str) -> bool {
    matches!(
        extension,
        "txt"
            | "md"
            | "json"
            | "csv"
            | "log"
            | "yaml"
            | "yml"
            | "toml"
            | "rs"
            | "ts"
            | "tsx"
            | "js"
            | "jsx"
            | "py"
            | "sh"
            | "css"
            | "xml"
            | "sql"
    )
}

/// `workspace.file_read`: safe read-only preview (text/image, honest degradation).
fn workspace_file_read(state: &Arc<AppState>, payload: &Value) -> CommandResponse {
    let request: zk_contract_ipc::payloads::WorkspaceFileReadPayload =
        match serde_json::from_value(payload.clone()) {
            Ok(request) => request,
            Err(error) => return CommandResponse::err(format!("workspace/invalid: {error}")),
        };
    let root = match workspace_root_of(state, &request.workspace_id) {
        Ok(root) => root,
        Err(error) => return CommandResponse::err(error),
    };
    let parts = match relative_parts(&request.path) {
        Ok(parts) => parts,
        Err(error) => return CommandResponse::err(error),
    };
    if parts.is_empty() {
        return CommandResponse::err("workspace/path-escape: empty preview path");
    }
    let target = match zk_composed::zk_kit_tools::safe_resolve_under(&root, &parts) {
        Ok(target) => target,
        Err(error) => {
            return CommandResponse::err(match error {
                zk_contract_tool::ToolError::NotFound(_) => "workspace/path-not-found".to_owned(),
                zk_contract_tool::ToolError::Denied(_)
                | zk_contract_tool::ToolError::InvalidArgs(_) => "workspace/path-escape".to_owned(),
                other => format!("workspace/path-unavailable: {other}"),
            });
        }
    };
    let metadata = match std::fs::metadata(&target) {
        Ok(metadata) => metadata,
        Err(error) => return CommandResponse::err(format!("workspace/path-not-found: {error}")),
    };
    if !metadata.is_file() {
        return CommandResponse::err("file/unsupported: not a regular file");
    }
    let size = metadata.len();
    let extension = file_extension(&request.path);
    let requested = request
        .max_bytes
        .unwrap_or(FILE_TEXT_MAX_BYTES)
        .clamp(1, FILE_TEXT_MAX_BYTES);
    let mut result = zk_contract_session::WorkspaceFileReadResult {
        path: request.path.clone(),
        kind: zk_contract_session::FileReadKind::Unsupported,
        mime: "application/octet-stream".to_owned(),
        size,
        truncated: false,
        encoding: String::new(),
        content: String::new(),
        reason: String::new(),
    };
    if is_image_extension(&extension) {
        if size > FILE_IMAGE_MAX_BYTES {
            result.reason = "file/too-large: image exceeds 8 MiB".to_owned();
            return CommandResponse::ok(json!(result));
        }
        let bytes = match std::fs::read(&target) {
            Ok(bytes) => bytes,
            Err(error) => {
                return CommandResponse::err(format!("workspace/path-unavailable: {error}"));
            }
        };
        result.kind = zk_contract_session::FileReadKind::Image;
        result.mime = match extension.as_str() {
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            "gif" => "image/gif",
            _ => "application/octet-stream",
        }
        .to_owned();
        result.encoding = "base64".to_owned();
        result.content = base64::engine::general_purpose::STANDARD.encode(bytes);
        return CommandResponse::ok(json!(result));
    }
    if !is_text_extension(&extension) {
        result.reason = if extension == "pdf" {
            "file/unsupported: PDF preview is not embedded".to_owned()
        } else if matches!(
            extension.as_str(),
            "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx"
        ) {
            "file/unsupported: office preview is not supported".to_owned()
        } else {
            "file/unsupported: binary type is not previewed".to_owned()
        };
        return CommandResponse::ok(json!(result));
    }
    let read_limit =
        usize::try_from(size.min(requested).min(size.saturating_add(1))).unwrap_or(usize::MAX);
    let mut bytes = Vec::new();
    let file = match std::fs::File::open(&target) {
        Ok(file) => file,
        Err(error) => return CommandResponse::err(format!("workspace/path-unavailable: {error}")),
    };
    if let Err(error) = std::io::Read::take(file, u64::try_from(read_limit).unwrap_or(u64::MAX))
        .read_to_end(&mut bytes)
    {
        return CommandResponse::err(format!("workspace/path-unavailable: {error}"));
    }
    let truncated = size > requested;
    let decode_budget = usize::try_from(requested).unwrap_or(usize::MAX);
    if bytes.len() > decode_budget {
        bytes.truncate(decode_budget);
    }
    let content = match String::from_utf8(bytes) {
        Ok(content) => content,
        Err(_) => {
            result.reason = "file/unsupported: not valid UTF-8".to_owned();
            return CommandResponse::ok(json!(result));
        }
    };
    result.kind = zk_contract_session::FileReadKind::Text;
    result.mime = "text/plain".to_owned();
    result.truncated = truncated;
    result.encoding = "utf8".to_owned();
    result.content = content;
    CommandResponse::ok(json!(result))
}

fn terminal_context(
    state: &Arc<AppState>,
    workspace_id: &str,
    conversation_id: &str,
) -> Result<TurnContext, String> {
    let conversations = state
        .runtime
        .sessions()
        .list_conversations_in(workspace_id)
        .map_err(|error| format!("workspace/not-found: {error}"))?;
    if !conversations
        .iter()
        .any(|conversation| conversation.id == conversation_id)
    {
        return Err("session/workspace-mismatch".to_owned());
    }
    let root = workspace_root_of(state, workspace_id)?;
    Ok(TurnContext {
        conversation_id: conversation_id.to_owned(),
        workspace_id: workspace_id.to_owned(),
        workspace_root: root.to_string_lossy().into_owned(),
    })
}

fn terminal_shell_kind() -> zk_composed::zk_kit_tools::persistent::ShellKind {
    if cfg!(windows) {
        zk_composed::zk_kit_tools::persistent::ShellKind::Pwsh
    } else {
        zk_composed::zk_kit_tools::persistent::ShellKind::Bash
    }
}

fn terminal_error(error: &zk_contract_tool::ToolError) -> String {
    let text = error.to_string();
    if text.contains("TOOL_UNSUPPORTED_PLATFORM") {
        format!("terminal/unavailable: {text}")
    } else if text.contains("timed out") || text.contains("timeout") {
        format!("terminal/timeout: {text}")
    } else if text.contains("session") {
        format!("terminal/session-invalid: {text}")
    } else {
        format!("terminal/failed: {text}")
    }
}

/// `terminal`: user-driven persistent shell (not an agent tool; no approvals).
fn terminal_command(state: &Arc<AppState>, payload: &Value) -> CommandResponse {
    let request: zk_contract_ipc::payloads::TerminalPayload =
        match serde_json::from_value(payload.clone()) {
            Ok(request) => request,
            Err(error) => return CommandResponse::err(format!("terminal/invalid: {error}")),
        };
    let context = match terminal_context(state, &request.workspace_id, &request.conversation_id) {
        Ok(context) => context,
        Err(error) => return CommandResponse::err(error),
    };
    let kind = terminal_shell_kind();
    use zk_composed::zk_kit_tools::persistent;
    let session = request
        .session
        .clone()
        .unwrap_or_else(|| persistent::DEFAULT_SESSION.to_owned());
    let sessions = persistent::list_sessions(&context, kind);
    let mut result = zk_contract_ipc::results::TerminalResult {
        sessions,
        output: String::new(),
        truncated: false,
        exit_code: None,
        timed_out: false,
    };
    match request.action {
        zk_contract_ipc::payloads::TerminalAction::Open => {
            if let Err(error) = persistent::open_session(&context, kind, &session) {
                return CommandResponse::err(terminal_error(&error));
            }
            result.sessions = persistent::list_sessions(&context, kind);
            CommandResponse::ok(json!(result))
        }
        zk_contract_ipc::payloads::TerminalAction::List => {
            result.sessions = persistent::list_sessions(&context, kind);
            CommandResponse::ok(json!(result))
        }
        zk_contract_ipc::payloads::TerminalAction::Status => {
            if !persistent::is_session_open(&context, kind, &session) {
                return CommandResponse::err("terminal/session-not-open");
            }
            result.sessions = persistent::list_sessions(&context, kind);
            CommandResponse::ok(json!(result))
        }
        zk_contract_ipc::payloads::TerminalAction::Close => {
            match persistent::close_session(&context, kind, &session) {
                Ok(_) => {
                    result.sessions = persistent::list_sessions(&context, kind);
                    CommandResponse::ok(json!(result))
                }
                Err(error) => CommandResponse::err(terminal_error(&error)),
            }
        }
        zk_contract_ipc::payloads::TerminalAction::Send => {
            let Some(command) = request.command.as_deref() else {
                return CommandResponse::err("terminal/session-invalid: missing command");
            };
            if command.trim().is_empty() {
                return CommandResponse::err("terminal/session-invalid: empty command");
            }
            if command.len() > TERMINAL_MAX_COMMAND_BYTES {
                return CommandResponse::err("terminal/session-invalid: command too long");
            }
            let timeout = request
                .timeout_ms
                .unwrap_or(TERMINAL_MAX_TIMEOUT_MS)
                .clamp(1, TERMINAL_MAX_TIMEOUT_MS);
            match persistent::run_in_session(&context, kind, &session, command, timeout) {
                Ok(output) => {
                    if output.len() >= TERMINAL_MAX_OUTPUT_BYTES {
                        // The persistent shell caps its own buffer at 64 KiB; mark it so the
                        // UI can show an honest truncation banner.
                        result.truncated = true;
                    }
                    let mut bytes = output.into_bytes();
                    if bytes.len() > TERMINAL_MAX_OUTPUT_BYTES {
                        bytes.truncate(TERMINAL_MAX_OUTPUT_BYTES);
                        result.truncated = true;
                    }
                    result.output = String::from_utf8_lossy(&bytes).into_owned();
                    result.sessions = persistent::list_sessions(&context, kind);
                    CommandResponse::ok(json!(result))
                }
                Err(error) => CommandResponse::err(terminal_error(&error)),
            }
        }
    }
}

/// `session.update`: rename / pin / archive one conversation (C18 workspace fixed).
fn session_update(state: &Arc<AppState>, payload: &Value) -> CommandResponse {
    let request: zk_contract_ipc::payloads::SessionUpdatePayload =
        match serde_json::from_value(payload.clone()) {
            Ok(request) => request,
            Err(error) => return CommandResponse::err(format!("session/invalid: {error}")),
        };
    let sessions = state.runtime.sessions();
    let mut conversation = None;
    if let Some(title) = request.title.as_deref() {
        match sessions.rename_conversation(&request.workspace_id, &request.conversation_id, title) {
            Ok(updated) => conversation = Some(updated),
            Err(error) => return session_err(error),
        }
    }
    if let Some(pinned) = request.pinned {
        match sessions.set_pinned(&request.workspace_id, &request.conversation_id, pinned) {
            Ok(updated) => conversation = Some(updated),
            Err(error) => return session_err(error),
        }
    }
    if let Some(archived) = request.archived {
        match sessions.set_archived(&request.workspace_id, &request.conversation_id, archived) {
            Ok(updated) => conversation = Some(updated),
            Err(error) => return session_err(error),
        }
    }
    match conversation {
        Some(conversation) => CommandResponse::ok(json!(conversation)),
        None => CommandResponse::err("session/invalid: no update field"),
    }
}

/// `session.fork`: copy one conversation inside its workspace (C18).
fn session_fork(state: &Arc<AppState>, payload: &Value) -> CommandResponse {
    let request: zk_contract_ipc::payloads::SessionForkPayload =
        match serde_json::from_value(payload.clone()) {
            Ok(request) => request,
            Err(error) => return CommandResponse::err(format!("session/invalid: {error}")),
        };
    match state.runtime.sessions().fork_conversation(
        &request.workspace_id,
        &request.conversation_id,
        request.title.as_deref(),
    ) {
        Ok(conversation) => CommandResponse::ok(json!(conversation)),
        Err(error) => session_err(error),
    }
}

/// `session.search`: current-workspace full-text search with snippet hits (C18).
fn session_search(state: &Arc<AppState>, payload: &Value) -> CommandResponse {
    let request: zk_contract_ipc::payloads::SessionSearchPayload =
        match serde_json::from_value(payload.clone()) {
            Ok(request) => request,
            Err(error) => return CommandResponse::err(format!("session/invalid: {error}")),
        };
    let query = request.query.trim();
    if query.is_empty() {
        return CommandResponse::ok(json!(zk_contract_session::SessionSearchResult {
            items: Vec::new(),
            truncated: false,
        }));
    }
    if query.chars().count() > 200 {
        return CommandResponse::err("session/invalid: query exceeds 200 chars");
    }
    let limit = request.limit.unwrap_or(20).clamp(1, 50);
    let sessions = state.runtime.sessions();
    match zk_composed::zk_kit_session::search(&*sessions, &request.workspace_id, query, limit) {
        Ok(result) => CommandResponse::ok(json!(result)),
        Err(error) => session_err(error),
    }
}

/// Composer attachment limit: one file, 1 MiB of raw bytes.
///
/// The serve carrier uses axum's default 2 MiB JSON body limit; 1 MiB raw bytes
/// become ~1.37 MiB as base64 + JSON envelope, so the default limit is enough
/// (`2026-11-01-client-composer-surface` D2 keeps `/ipc` unchanged).
const MAX_ATTACHMENT_BYTES: usize = 1024 * 1024;
/// Sanitized attachment display name limit (bytes).
const MAX_ATTACHMENT_NAME_BYTES: usize = 128;
/// Host attachment extension allow-list (8 kinds; design §2.2).
const ATTACHMENT_EXT_ALLOWLIST: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "gif", "pdf", "txt", "md", "json", "csv", "log",
];

/// `workspace.attachment_save`: validate + persist one composer attachment.
///
/// The client hints are never trusted: workspace/conversation ownership, the
/// filename (basename only), decode size, extension allow-list, magic bytes and
/// the canonical MIME are re-derived here. The write path is built exclusively
/// through the generated `zk_composed::zk_kit_tools::safe_join_under` facade
/// (no manual joining; C17/C4), which rejects `..`/absolute parts and any
/// symlinked parent or file component.
fn attachment_save(state: &Arc<AppState>, payload: &Value) -> CommandResponse {
    let request: zk_contract_ipc::payloads::AttachmentSavePayload =
        match serde_json::from_value(payload.clone()) {
            Ok(request) => request,
            Err(error) => return CommandResponse::err(format!("attachment/invalid: {error}")),
        };
    // The client MIME is only a hint; the host derives the canonical one.
    let _client_mime = request.mime.as_str();
    let sessions = state.runtime.sessions();
    let conversations = match sessions.list_conversations_in(&request.workspace_id) {
        Ok(list) => list,
        Err(error) => return CommandResponse::err(format!("workspace/not-found: {error}")),
    };
    if !conversations
        .iter()
        .any(|conversation| conversation.id == request.conversation_id)
    {
        return CommandResponse::err("session/workspace-mismatch");
    }
    let workspaces = match sessions.list_workspaces() {
        Ok(list) => list,
        Err(error) => return CommandResponse::err(error.to_string()),
    };
    let Some(workspace) = workspaces
        .into_iter()
        .find(|workspace| workspace.id == request.workspace_id)
    else {
        return CommandResponse::err("workspace/not-found");
    };
    let name = match sanitize_attachment_name(&request.name) {
        Ok(name) => name,
        Err(error) => return CommandResponse::err(error),
    };
    let Some(extension) = attachment_extension(&name) else {
        return CommandResponse::err("attachment/type-not-allowed: missing extension");
    };
    if !ATTACHMENT_EXT_ALLOWLIST.contains(&extension.as_str()) {
        return CommandResponse::err(format!("attachment/type-not-allowed: {extension}"));
    }
    let bytes =
        match base64::engine::general_purpose::STANDARD.decode(request.data_base64.as_bytes()) {
            Ok(bytes) => bytes,
            Err(_) => return CommandResponse::err("attachment/invalid: base64"),
        };
    if bytes.is_empty() {
        return CommandResponse::err("attachment/empty");
    }
    if bytes.len() > MAX_ATTACHMENT_BYTES {
        return CommandResponse::err("attachment/too-large");
    }
    let mime = match attachment_mime(&extension, &bytes) {
        Ok(mime) => mime,
        Err(error) => return CommandResponse::err(error),
    };
    let file_name = format!("{}-{name}", uuid::Uuid::new_v4());
    let joined = match zk_composed::zk_kit_tools::safe_join_under(
        std::path::Path::new(&workspace.path),
        &[
            ".zeokeel",
            "attachments",
            request.conversation_id.as_str(),
            file_name.as_str(),
        ],
    ) {
        Ok(path) => path,
        Err(error) => {
            return CommandResponse::err(format!("attachment/workspace-unavailable: {error}"));
        }
    };
    let tmp = joined.with_file_name(format!(".{file_name}.tmp"));
    if let Err(error) = write_attachment_atomic(&joined, &tmp, &bytes) {
        return CommandResponse::err(format!("attachment/workspace-unavailable: {error}"));
    }
    let relative = format!(
        ".zeokeel/attachments/{}/{}",
        request.conversation_id, file_name
    );
    CommandResponse::ok(json!(zk_contract_ipc::results::AttachmentSaveResult {
        path: relative,
        name,
        bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        mime,
    }))
}

/// Sanitize the client-declared file name (basename only; no separators).
fn sanitize_attachment_name(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("attachment/invalid: empty file name".to_owned());
    }
    if trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains("..")
    {
        return Err("attachment/invalid: unsafe file name".to_owned());
    }
    if trimmed.chars().any(char::is_control) {
        return Err("attachment/invalid: control character in file name".to_owned());
    }
    if trimmed.len() > MAX_ATTACHMENT_NAME_BYTES {
        return Err("attachment/invalid: file name too long".to_owned());
    }
    Ok(trimmed.to_owned())
}

/// Lowercase extension of a sanitized file name (`None` when there is none).
fn attachment_extension(name: &str) -> Option<String> {
    let (_, extension) = name.rsplit_once('.')?;
    if extension.is_empty() {
        return None;
    }
    Some(extension.to_ascii_lowercase())
}

/// Canonical MIME for an allowed extension, validating the file magic/content.
fn attachment_mime(extension: &str, bytes: &[u8]) -> Result<String, String> {
    let (mime, matches) = match extension {
        "png" => ("image/png", bytes.starts_with(b"\x89PNG\r\n\x1a\n")),
        "jpg" | "jpeg" => ("image/jpeg", bytes.starts_with(&[0xFF, 0xD8, 0xFF])),
        "gif" => (
            "image/gif",
            bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        ),
        "webp" => (
            "image/webp",
            bytes.len() >= 12
                && bytes.starts_with(b"RIFF")
                && bytes.get(8..12) == Some(b"WEBP".as_slice()),
        ),
        "pdf" => ("application/pdf", bytes.starts_with(b"%PDF-")),
        "txt" | "log" => ("text/plain", is_attachment_text(bytes)),
        "md" => ("text/markdown", is_attachment_text(bytes)),
        "json" => ("application/json", is_attachment_text(bytes)),
        "csv" => ("text/csv", is_attachment_text(bytes)),
        _ => return Err("attachment/type-not-allowed: extension".to_owned()),
    };
    if !matches {
        return Err(format!(
            "attachment/invalid: content does not match .{extension}"
        ));
    }
    Ok(mime.to_owned())
}

/// Text attachments must be valid UTF-8 without NUL bytes (bounded by the caller).
fn is_attachment_text(bytes: &[u8]) -> bool {
    !bytes.contains(&0u8) && std::str::from_utf8(bytes).is_ok()
}

/// Write the attachment atomically (tmp file in the same validated directory).
fn write_attachment_atomic(
    path: &std::path::Path,
    tmp: &std::path::Path,
    bytes: &[u8],
) -> Result<(), String> {
    let mut file = std::fs::File::create(tmp).map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(tmp, std::fs::Permissions::from_mode(0o600))
            .map_err(|error| error.to_string())?;
    }
    std::fs::rename(tmp, path).map_err(|error| {
        let _ = std::fs::remove_file(tmp);
        error.to_string()
    })
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

pub(crate) fn open_platform_browser(url: &str) -> Result<(), String> {
    let (program, args) = open_url_invocation(url)?;
    let mut command = std::process::Command::new(program);
    command
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    spawn_and_reap(command)
}

/// Validate an OAuth authorize URL, then open it in the system browser (no wait).
///
/// Reuses the `system.open_url` validation (absolute http(s), no credentials,
/// no fragment) so the OAuth flow can never hand the platform opener an
/// unexpected scheme.
pub(crate) fn open_platform_browser_checked(url: &str) -> Result<(), String> {
    let url = validate_open_url(url)?;
    open_platform_browser(&url)
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

fn valid_connector_id(connector_id: &str) -> bool {
    !connector_id.is_empty()
        && connector_id.len() <= 128
        && connector_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

fn oauth_entry(connector_id: &str) -> Result<keyring::Entry, String> {
    if !valid_connector_id(connector_id) {
        return Err("OAUTH_VALIDATION".to_owned());
    }
    keyring::Entry::new("zeokeel-connector-oauth", connector_id)
        .map_err(|error| format!("keychain: {error}"))
}

/// Persist one connector's token JSON (OS keychain; dev file fallback in tests).
fn oauth_save_tokens(connector_id: &str, tokens: &Value) -> Result<(), String> {
    if !tokens.is_object() {
        return Err("OAUTH_VALIDATION".to_owned());
    }
    let entry = match oauth_entry(connector_id) {
        Ok(entry) => entry,
        Err(_) if dev_oauth_fallback() => {
            let mut map = read_dev_oauth();
            map.insert(connector_id.to_owned(), tokens.clone());
            return write_dev_oauth(&map);
        }
        Err(error) => return Err(error),
    };
    match entry.set_password(&tokens.to_string()) {
        Ok(()) => Ok(()),
        Err(_error) if dev_oauth_fallback() => {
            let mut map = read_dev_oauth();
            map.insert(connector_id.to_owned(), tokens.clone());
            write_dev_oauth(&map)
        }
        Err(error) => Err(format!("keychain: {error}")),
    }
}

/// Load one connector's raw token JSON string (never logged).
fn oauth_stored_raw(connector_id: &str) -> Result<Option<String>, String> {
    let entry = match oauth_entry(connector_id) {
        Ok(entry) => entry,
        Err(_) if dev_oauth_fallback() => {
            return Ok(read_dev_oauth()
                .get(connector_id)
                .map(std::string::ToString::to_string));
        }
        Err(error) => return Err(error),
    };
    match entry.get_password() {
        Ok(raw) => Ok(Some(raw)),
        Err(_) if dev_oauth_fallback() => Ok(read_dev_oauth()
            .get(connector_id)
            .map(std::string::ToString::to_string)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!("keychain: {error}")),
    }
}

fn oauth_save(payload: &Value) -> CommandResponse {
    let (Some(connector_id), Some(tokens)) = (
        payload.get("connector_id").and_then(Value::as_str),
        payload.get("tokens"),
    ) else {
        return CommandResponse::err("missing connector_id/tokens");
    };
    match oauth_save_tokens(connector_id, tokens) {
        Ok(()) => CommandResponse::ok(json!({ "saved": true })),
        Err(error) => CommandResponse::err(error),
    }
}

fn oauth_load(payload: &Value) -> CommandResponse {
    let Some(connector_id) = payload.get("connector_id").and_then(Value::as_str) else {
        return CommandResponse::err("missing connector_id");
    };
    match oauth_stored_raw(connector_id) {
        Ok(raw) => CommandResponse::ok(json!({ "tokens": raw })),
        Err(error) => CommandResponse::err(error),
    }
}

fn oauth_clear(payload: &Value) -> CommandResponse {
    let Some(connector_id) = payload.get("connector_id").and_then(Value::as_str) else {
        return CommandResponse::err("missing connector_id");
    };
    // Disconnect path: drop the in-memory MCP bearer token first, whichever
    // keychain backend (real or dev fallback) handles the stored copy.
    let _ = zk_connector_mcp::set_oauth_token(connector_id, None);
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

/// Read + validate the OAuth public-client metadata from an IPC payload.
fn oauth_flow_config(
    payload: &Value,
) -> Result<(String, zk_contract_connector::ConnectorOAuth, u64), CommandResponse> {
    let Some(connector_id) = payload.get("connector_id").and_then(Value::as_str) else {
        return Err(CommandResponse::err("missing connector_id"));
    };
    if !valid_connector_id(connector_id) {
        return Err(CommandResponse::err("OAUTH_VALIDATION"));
    }
    let Some(oauth) = payload.get("oauth") else {
        return Err(CommandResponse::err("missing oauth metadata"));
    };
    let config: zk_contract_connector::ConnectorOAuth = serde_json::from_value(oauth.clone())
        .map_err(|_| CommandResponse::err("OAUTH_VALIDATION: oauth metadata shape"))?;
    config.validate().map_err(CommandResponse::err)?;
    let timeout_seconds = payload
        .get("timeout_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(crate::mcp_oauth::DEFAULT_TIMEOUT_SECONDS)
        .clamp(
            crate::mcp_oauth::MIN_TIMEOUT_SECONDS,
            crate::mcp_oauth::MAX_TIMEOUT_SECONDS,
        );
    Ok((connector_id.to_owned(), config, timeout_seconds))
}

fn oauth_flow_set(state: &AppState, connector_id: &str, status: OAuthFlowStatus) {
    if let Ok(mut flows) = state.oauth_flows.lock() {
        flows.insert(connector_id.to_owned(), status);
    }
}

fn oauth_flow_pending(state: &AppState, connector_id: &str) -> bool {
    state
        .oauth_flows
        .lock()
        .ok()
        .and_then(|flows| flows.get(connector_id).cloned())
        .is_some_and(|status| status.state == "pending")
}

/// `oauth.authorize`: start the loopback + PKCE flow in the background.
///
/// Returns `{started:true}` immediately (the browser flow can take minutes);
/// the UI polls [`oauth_status`] — which never returns token material.
fn oauth_authorize(state: &Arc<AppState>, payload: &Value) -> CommandResponse {
    let (connector_id, config, timeout_seconds) = match oauth_flow_config(payload) {
        Ok(parsed) => parsed,
        Err(error) => return error,
    };
    if oauth_flow_pending(state, &connector_id) {
        return CommandResponse::err("OAUTH_ALREADY_PENDING");
    }
    oauth_flow_set(
        state,
        &connector_id,
        OAuthFlowStatus {
            state: "pending".to_owned(),
            error: None,
        },
    );
    let thread_state = Arc::clone(state);
    let thread_id = connector_id.clone();
    std::thread::spawn(move || {
        let result = crate::mcp_oauth::authorize(
            &config,
            std::time::Duration::from_secs(timeout_seconds),
            crate::mcp_oauth::now_ms(),
            &crate::mcp_oauth::open_system_browser,
        );
        let status = match result {
            Ok(tokens) => match oauth_save_tokens(&thread_id, &tokens.to_json()) {
                Ok(()) => {
                    // Hand the bearer token to the MCP transport (in-memory only).
                    let _ = zk_connector_mcp::set_oauth_token(
                        &thread_id,
                        Some(tokens.access_token.clone()),
                    );
                    OAuthFlowStatus {
                        state: "authorized".to_owned(),
                        error: None,
                    }
                }
                Err(error) => OAuthFlowStatus {
                    state: "failed".to_owned(),
                    error: Some(error),
                },
            },
            Err(error) => OAuthFlowStatus {
                state: "failed".to_owned(),
                error: Some(error),
            },
        };
        oauth_flow_set(&thread_state, &thread_id, status);
    });
    CommandResponse::ok(json!({
        "started": true,
        "connector_id": connector_id,
        "timeout_seconds": timeout_seconds,
    }))
}

/// `oauth.status`: poll one connector's flow state (`idle|pending|authorized|failed`).
fn oauth_status(state: &AppState, payload: &Value) -> CommandResponse {
    let Some(connector_id) = payload.get("connector_id").and_then(Value::as_str) else {
        return CommandResponse::err("missing connector_id");
    };
    if !valid_connector_id(connector_id) {
        return CommandResponse::err("OAUTH_VALIDATION");
    }
    match state
        .oauth_flows
        .lock()
        .ok()
        .and_then(|flows| flows.get(connector_id).cloned())
    {
        Some(status) => CommandResponse::ok(json!({
            "state": status.state,
            "error": status.error,
        })),
        None => CommandResponse::ok(json!({ "state": "idle", "error": null })),
    }
}

/// `oauth.refresh`: rotate the stored token in the background (poll `oauth.status`).
///
/// Refresh happens even when the stored token is not yet near expiry: the UI calls
/// this on boot/reconnect, and providers may have invalidated the access token early.
fn oauth_refresh(state: &Arc<AppState>, payload: &Value) -> CommandResponse {
    let (connector_id, config, _timeout_seconds) = match oauth_flow_config(payload) {
        Ok(parsed) => parsed,
        Err(error) => return error,
    };
    if oauth_flow_pending(state, &connector_id) {
        return CommandResponse::err("OAUTH_ALREADY_PENDING");
    }
    let raw = match oauth_stored_raw(&connector_id) {
        Ok(Some(raw)) => raw,
        Ok(None) => return CommandResponse::err("OAUTH_NO_TOKEN"),
        Err(error) => return CommandResponse::err(error),
    };
    let tokens = match crate::mcp_oauth::OAuthTokens::from_stored_json(&raw) {
        Ok(tokens) => tokens,
        Err(error) => return CommandResponse::err(error),
    };
    oauth_flow_set(
        state,
        &connector_id,
        OAuthFlowStatus {
            state: "pending".to_owned(),
            error: None,
        },
    );
    let thread_state = Arc::clone(state);
    let thread_id = connector_id.clone();
    std::thread::spawn(move || {
        let result = crate::mcp_oauth::refresh(&config, &tokens, crate::mcp_oauth::now_ms());
        let status = match result {
            Ok(next) => match oauth_save_tokens(&thread_id, &next.to_json()) {
                Ok(()) => {
                    let _ = zk_connector_mcp::set_oauth_token(
                        &thread_id,
                        Some(next.access_token.clone()),
                    );
                    OAuthFlowStatus {
                        state: "authorized".to_owned(),
                        error: None,
                    }
                }
                Err(error) => OAuthFlowStatus {
                    state: "failed".to_owned(),
                    error: Some(error),
                },
            },
            Err(error) => OAuthFlowStatus {
                state: "failed".to_owned(),
                error: Some(error),
            },
        };
        oauth_flow_set(&thread_state, &thread_id, status);
    });
    CommandResponse::ok(json!({ "started": true, "connector_id": connector_id }))
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
    if let Some(media) = value.get_mut("media").and_then(Value::as_object_mut) {
        media.insert("api_key".to_owned(), Value::String(String::new()));
    }
    value
}

/// Readable revision-conflict error. The `settings/revision-conflict` prefix is
/// the stable code the UI maps to its i18n message (nothing is written).
fn settings_conflict_error(expected: u64, current: u64) -> String {
    format!("settings/revision-conflict: expected revision {expected}, current revision {current}")
}

/// Whether an embedding section carries any non-default configuration.
fn embedding_attempted(embedding: &zk_contract_config::EmbeddingSettings) -> bool {
    embedding.is_configured()
        || !embedding.base_url.trim().is_empty()
        || !embedding.model.trim().is_empty()
        || embedding.dims.is_some()
        || !embedding.dir.trim().is_empty()
}

/// Enterprise (C7) never accepts a local embedding provider; personal edition
/// does. Pure helper so both editions are unit-testable.
fn embedding_edition_rejection(
    edition: &str,
    embedding: &zk_contract_config::EmbeddingSettings,
) -> Option<String> {
    if edition == "personal" {
        return None;
    }
    embedding_attempted(embedding).then(|| {
        "embedding provider is personal-edition only; enterprise AI must use the gateway (C7)"
            .to_owned()
    })
}

/// Apply the embedding provider settings.
///
/// Personal edition validates and pushes the config into the generated glue
/// (errors propagate to the caller: `settings.set` must not persist a failed
/// config, boot records it as `agent_config_error`). Enterprise rejects any
/// attempted local provider and otherwise no-ops (C7).
pub(crate) fn apply_embedding_settings(settings: &AppSettings) -> Result<(), String> {
    settings
        .embedding
        .validate()
        .map_err(|error| format!("embedding settings: {error}"))?;
    if let Some(rejection) = embedding_edition_rejection(zk_composed::EDITION, &settings.embedding)
    {
        return Err(rejection);
    }
    if zk_composed::EDITION == "personal" {
        zk_composed::configure_embedding_from_settings(&settings.embedding)
            .map_err(|error| format!("embedding settings: {error}"))?;
    }
    Ok(())
}

/// Whether a media section carries any non-default configuration.
fn media_attempted(media: &zk_contract_config::MediaSettings) -> bool {
    media.is_configured()
        || !media.base_url.trim().is_empty()
        || !media.image_model.trim().is_empty()
        || !media.audio_model.trim().is_empty()
        || !media.transcription_model.trim().is_empty()
        || !media.rerank_model.trim().is_empty()
}

/// Enterprise (C7) never accepts local media providers; personal edition does.
fn media_edition_rejection(
    edition: &str,
    media: &zk_contract_config::MediaSettings,
) -> Option<String> {
    if edition == "personal" {
        return None;
    }
    media_attempted(media).then(|| {
        "media providers are personal-edition only; enterprise AI must use the gateway (C7)"
            .to_owned()
    })
}

/// Apply the media capability settings (image/audio/transcription/rerank).
///
/// Personal edition validates and pushes the config into the generated glue;
/// errors propagate so `settings.set` rejects without saving and boot records
/// `agent_config_error`. Enterprise rejects any attempted local provider and
/// otherwise no-ops (C7).
pub(crate) fn apply_media_settings(settings: &AppSettings) -> Result<(), String> {
    settings
        .media
        .validate()
        .map_err(|error| format!("media settings: {error}"))?;
    if let Some(rejection) = media_edition_rejection(zk_composed::EDITION, &settings.media) {
        return Err(rejection);
    }
    if zk_composed::EDITION == "personal" {
        zk_composed::configure_media_from_settings(&settings.media)
            .map_err(|error| format!("media settings: {error}"))?;
    }
    Ok(())
}

/// Keep stored write-only secrets when the UI sends a masked (empty) value.
///
/// The embedding key is only kept for personal edition: enterprise must never
/// retain an upstream embedding key (C7).
fn keep_existing_secrets(settings: &mut AppSettings, existing: Option<&AppSettings>) {
    let Some(existing) = existing else {
        return;
    };
    if settings.provider.api_key.trim().is_empty() {
        settings.provider.api_key = existing.provider.api_key.clone();
    }
    if zk_composed::EDITION == "personal" && settings.embedding.api_key.trim().is_empty() {
        settings.embedding.api_key = existing.embedding.api_key.clone();
    }
    if zk_composed::EDITION == "personal" && settings.media.api_key.trim().is_empty() {
        settings.media.api_key = existing.media.api_key.clone();
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
    if let Err(error) = settings.media.validate() {
        return Err(error.to_string());
    }
    if let Err(error) = settings.web_search.validate() {
        return Err(error.to_string());
    }
    if let Err(error) = settings.mcp.validate() {
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
        turn_id: String::new(),
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
    // Mark the conversation as running so `/compact` can reject it (`compact/busy`).
    state.mark_conversation_running(&job.conversation_id);
    struct RunningGuard<'a> {
        state: &'a Arc<AppState>,
        conversation_id: &'a str,
    }
    impl Drop for RunningGuard<'_> {
        fn drop(&mut self) {
            self.state.unmark_conversation_running(self.conversation_id);
        }
    }
    let _running_guard = RunningGuard {
        state,
        conversation_id: &job.conversation_id,
    };
    // Plan mode is read from `.zeokeel/plans.json` at turn start; agent-rig then
    // blocks mutating/delegation tools for this turn only.
    let plan_root = state
        .runtime()
        .sessions()
        .list_workspaces()
        .ok()
        .and_then(|workspaces| {
            workspaces
                .into_iter()
                .find(|workspace| workspace.id == job.workspace_id)
                .map(|workspace| workspace.path)
        })
        .unwrap_or_default();
    let plan_active = crate::plan::plan_mode_active(state, &plan_root, &job.conversation_id);
    state.runtime().set_plan_mode(plan_active);
    struct PlanModeReset<'a> {
        state: &'a Arc<AppState>,
    }
    impl Drop for PlanModeReset<'_> {
        fn drop(&mut self) {
            self.state.runtime().set_plan_mode(false);
        }
    }
    let _plan_reset = PlanModeReset { state };
    // Global config mutex: one turn at a time may mutate the shared agent config
    // (model/agent/approval override). Queued turns cancelled before acquiring the
    // lock are skipped without events/errors (spec R4 P2-1).
    let _config_guard = crate::command_surface::lock_config();
    if job.cancelled.load(Ordering::SeqCst) {
        if let Ok(mut streams) = state.streams.lock() {
            streams.remove(&job.stream_id);
        }
        return;
    }
    let global_settings =
        crate::command_surface::snapshot_settings(&*state.runtime.settings()).unwrap_or_default();
    if let Err(error) = crate::command_surface::apply_override(
        &state.runtime,
        &global_settings,
        &job.workspace_id,
        &job.conversation_id,
    ) {
        let _ = crate::command_surface::restore_global(&state.runtime, &global_settings);
        if let Ok(mut streams) = state.streams.lock() {
            streams.remove(&job.stream_id);
        }
        let payload = serde_json::to_value(zk_contract_agent::AgentEvent::Error { message: error })
            .unwrap_or(Value::Null);
        state
            .bus
            .send(UiEvent {
                stream_id: job.stream_id.clone(),
                seq: 1,
                kind: "agent".to_owned(),
                payload,
            })
            .ok();
        return;
    }
    let seq = Arc::new(AtomicU64::new(1));
    let bridge = Arc::new(BridgeSink::new(
        state.bus.clone(),
        job.stream_id.clone(),
        Arc::clone(&seq),
        Arc::clone(&job.cancelled),
        state.runtime.sessions(),
        job.conversation_id.clone(),
    ));
    let sink: Arc<dyn zk_contract_agent::EventSink> = bridge.clone();
    let context = TurnContext {
        conversation_id: job.conversation_id.clone(),
        workspace_id: job.workspace_id.clone(),
        workspace_root: job.workspace_root.clone(),
    };
    // B8: SQLite session store is the replay authority. Drop the just-persisted current
    // user message so the prompt is not duplicated; tool rows are text-only and are not
    // replayed (rig history needs paired tool-call ids).
    let raw = state
        .runtime
        .sessions()
        .messages_in(&job.workspace_id, &job.conversation_id)
        .unwrap_or_default();
    // Replay uses the compacted view (marker summary + recent raw messages); the
    // store itself keeps raw history for UI/search/fork/audit.
    let replay = crate::compact::compacted_messages(&raw);
    let history = replay_history(&replay, &job.text);
    let result = state.runtime.run_stream_in_shared_with_history(
        &context,
        &job.stream_id,
        &job.text,
        &history,
        sink,
    );
    // Restore the global agent config from the turn-start snapshot (still under
    // the config mutex; a concurrent settings.set applies its own snapshot later).
    if let Err(error) = crate::command_surface::restore_global(&state.runtime, &global_settings) {
        eprintln!("zk: restore global settings after turn failed: {error}");
    }
    drop(_config_guard);
    match result {
        Ok(reply) => {
            let status = if bridge.turn_cancelled() {
                "cancelled"
            } else {
                "done"
            };
            let message = assistant_message(
                &job.conversation_id,
                reply.text,
                reply.usage,
                status,
                &bridge.turn_id(),
            );
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
                turn_id: bridge.turn_id(),
            };
            let _ = state.runtime.sessions().append_message(&message);
        }
    }
    if let Ok(mut streams) = state.streams.lock() {
        streams.remove(&job.stream_id);
    }
}

/// Maximum replayed history messages per turn (keeps the prompt bounded).
const MAX_REPLAY_MESSAGES: usize = 40;

/// Map persisted session messages to prompt history for replay (B8).
///
/// Only `user`/`assistant` text rows are replayed; the current prompt (already
/// persisted as the last user row) is dropped, and the tail is capped so a long
/// conversation cannot grow the prompt without bound.
/// Render one compaction marker as an injected user-visible history summary.
fn marker_summary(message: &Message) -> Option<String> {
    if message.role != "system" || !message.id.starts_with(crate::compact::MARKER_PREFIX) {
        return None;
    }
    let content = zk_composed::zk_kit_compaction::MarkerContent::parse(&message.content)?;
    Some(format!(
        "[Earlier conversation summary ({0} messages compressed)]\n{1}",
        content.compressed_messages, content.summary
    ))
}

fn replay_history(messages: &[Message], current_input: &str) -> Vec<HistoryMessage> {
    let mut history: Vec<HistoryMessage> = Vec::new();
    for message in messages {
        if let Some(content) = marker_summary(message) {
            history.push(HistoryMessage {
                role: HistoryRole::User,
                content,
            });
            continue;
        }
        match message.role.as_str() {
            "user" => history.push(HistoryMessage {
                role: HistoryRole::User,
                content: message.content.clone(),
            }),
            "assistant" if message.status != "error" => history.push(HistoryMessage {
                role: HistoryRole::Assistant,
                content: message.content.clone(),
            }),
            _ => {}
        }
    }
    let current_is_last = history
        .last()
        .is_some_and(|last| last.role == HistoryRole::User && last.content == current_input);
    if current_is_last {
        history.pop();
    }
    let len = history.len();
    if len > MAX_REPLAY_MESSAGES {
        history = history.split_off(len.saturating_sub(MAX_REPLAY_MESSAGES));
    }
    history
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
    turn_id: &str,
) -> Message {
    Message {
        id: uuid::Uuid::new_v4().to_string(),
        conversation_id: conversation_id.to_owned(),
        role: "assistant".to_owned(),
        content: text,
        created_at_ms: now_ms(),
        status: status.to_owned(),
        usage,
        turn_id: turn_id.to_owned(),
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
    // Release any pending approval waiters so the tool closure can observe the
    // cancel flag instead of blocking until the approval timeout, then flip the
    // shared flags. The agent runtime is the single TurnCancelled emitter.
    state.approvals.release_all(ApprovalDecision::Reject);
    flag.store(true, Ordering::SeqCst);
    state.runtime.cancel_agent();
    CommandResponse::ok(json!({ "cancelled": true }))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only
    use base64::Engine;
    use serde_json::{json, Value};
    use std::io::Write;

    /// Serializes tests that mutate the process-wide `ZEOKEEL_HOME` (settings
    /// store/session DB resolution happens at runtime-build time).
    static HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
        settings.web_search.base_url = "https://search.example.invalid/".to_owned();
        settings.web_search.timeout_ms = 20_000;
        super::publish_runtime_settings(&super::runtime_snapshot_for(&settings, true, true, true))
            .unwrap();

        let published = zk_contract_config::runtime_settings().unwrap().unwrap();
        assert_eq!(published.agent.as_ref().unwrap().max_steps, 7);
        assert_eq!(published.tools.unwrap().tool_timeout_ms, 5_000);
        let published_search = published.web_search.clone().unwrap();
        assert_eq!(published_search.base_url, "https://search.example.invalid/");
        assert_eq!(published_search.timeout_ms, 20_000);
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
        let empty = super::runtime_snapshot_for(&defaults, true, true, true);
        assert!(empty.agent.is_none(), "default agent section not published");
        assert!(empty.tools.is_none(), "default tools section not published");
        assert!(
            empty.web_search.is_none(),
            "default web_search section not published"
        );

        let mut tools_only = zk_contract_config::AppSettings::default();
        tools_only.tools.tool_timeout_ms = 5_000;
        let snapshot = super::runtime_snapshot_for(&tools_only, true, true, true);
        assert!(
            snapshot.agent.is_none(),
            "tools-only save must not publish agent defaults"
        );
        assert_eq!(snapshot.tools.unwrap().tool_timeout_ms, 5_000);
        assert!(snapshot.web_search.is_none());

        let mut agent_only = zk_contract_config::AppSettings::default();
        agent_only.agent.max_steps = 12;
        let snapshot = super::runtime_snapshot_for(&agent_only, true, true, true);
        assert_eq!(snapshot.agent.unwrap().max_steps, 12);
        assert!(
            snapshot.tools.is_none(),
            "agent-only save must not publish tool defaults"
        );
        assert!(snapshot.web_search.is_none());

        // Web-search-only saves publish just that section (settings > env > default).
        let mut search_only = zk_contract_config::AppSettings::default();
        search_only.web_search.base_url = "https://search.example.invalid/".to_owned();
        search_only.web_search.timeout_ms = 30_000;
        let snapshot = super::runtime_snapshot_for(&search_only, true, true, true);
        assert!(snapshot.agent.is_none() && snapshot.tools.is_none());
        let published = snapshot.web_search.unwrap();
        assert_eq!(published.base_url, "https://search.example.invalid/");
        assert_eq!(published.timeout_ms, 30_000);

        // Boot must respect file existence for every section.
        let nothing = super::runtime_snapshot_for(&agent_only, false, false, false);
        assert!(nothing.agent.is_none() && nothing.tools.is_none() && nothing.web_search.is_none());
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
    fn invalid_web_search_settings_are_rejected_by_validate_settings() {
        let mut invalid = zk_contract_config::AppSettings::default();
        invalid.web_search.base_url = "http://search.example.invalid".to_owned();
        assert!(super::validate_settings(&invalid).is_err());
        invalid.web_search.base_url = "https://search.example.invalid/".to_owned();
        invalid.web_search.timeout_ms = 500;
        assert!(super::validate_settings(&invalid).is_err());
        invalid.web_search.timeout_ms = 120_001;
        assert!(super::validate_settings(&invalid).is_err());
        invalid.web_search.timeout_ms = 15_000;
        assert!(super::validate_settings(&invalid).is_ok());
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
        let with_usage =
            super::assistant_message("c1", "answer".to_owned(), Some(usage), "done", "turn-1");
        assert_eq!(with_usage.usage, Some(usage));
        assert_eq!(with_usage.status, "done");
        assert_eq!(with_usage.role, "assistant");
        let without =
            super::assistant_message("c1", "answer".to_owned(), None, "cancelled", "turn-2");
        assert!(without.usage.is_none());
        assert_eq!(without.status, "cancelled");
    }

    #[test]
    fn media_settings_apply_mask_validation_and_edition() {
        let settings = zk_contract_config::AppSettings {
            media: zk_contract_config::MediaSettings {
                base_url: "https://api.example.invalid/v1".to_owned(),
                api_key: "sk-media-secret".to_owned(),
                image_model: "gpt-image-1".to_owned(),
                audio_model: String::new(),
                transcription_model: "whisper-1".to_owned(),
                rerank_model: "rerank-2.5".to_owned(),
            },
            ..zk_contract_config::AppSettings::default()
        };
        super::apply_media_settings(&settings).unwrap();

        // settings.get masks the write-only media key (like provider/embedding).
        let masked = super::masked_settings(&settings);
        assert_eq!(masked["media"]["api_key"], "");
        assert_eq!(masked["media"]["image_model"], "gpt-image-1");
        assert_eq!(masked["embedding"]["api_key"], "");

        // A masked (empty) incoming key keeps the stored secret on save.
        let mut incoming = settings.clone();
        incoming.media.api_key = String::new();
        super::keep_existing_secrets(&mut incoming, Some(&settings));
        assert_eq!(incoming.media.api_key, "sk-media-secret");
        assert_eq!(super::masked_settings(&incoming)["media"]["api_key"], "");

        // Invalid shared base URL is rejected before persisting/applying.
        let mut invalid = settings.clone();
        invalid.media.base_url = "ftp://example.invalid".to_owned();
        assert!(super::apply_media_settings(&invalid).is_err());
        assert!(super::validate_settings(&invalid).is_err());

        // Enterprise (C7) rejects any attempted local media provider.
        assert!(super::media_edition_rejection("enterprise", &settings.media).is_some());
        assert!(super::media_edition_rejection(
            "enterprise",
            &zk_contract_config::MediaSettings::default()
        )
        .is_none());
        assert!(super::media_edition_rejection("personal", &settings.media).is_none());
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

        // Glue-level failure (key present, model empty) propagates so
        // settings.set can reject and skip the save.
        let mut missing_model = settings.clone();
        missing_model.embedding.model = String::new();
        assert!(super::apply_embedding_settings(&missing_model).is_err());

        // Enterprise (C7) rejects any attempted local embedding provider.
        assert!(super::embedding_edition_rejection("enterprise", &settings.embedding).is_some());
        assert!(super::embedding_edition_rejection(
            "enterprise",
            &zk_contract_config::EmbeddingSettings::default()
        )
        .is_none());
        assert!(super::embedding_edition_rejection("personal", &settings.embedding).is_none());
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
    fn settings_set_rejects_failed_embedding_without_persisting() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-settings-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", &home);
        let state = super::AppState::new(zk_composed::build().expect("composed build"));

        // Establish the persisted settings file with defaults.
        let saved = super::dispatch(
            &state,
            zk_contract_ipc::commands::SETTINGS_SET,
            &serde_json::json!({"settings": zk_contract_config::AppSettings::default()}),
        );
        assert!(saved.ok, "{:?}", saved.error);
        let before = state.runtime().settings().load().unwrap();
        assert_eq!(
            before.embedding,
            zk_contract_config::EmbeddingSettings::default()
        );

        // Configured key + empty model: validation passes but the glue fails.
        let mut invalid = before.clone();
        invalid.embedding = zk_contract_config::EmbeddingSettings {
            base_url: "https://api.example.invalid/v1".to_owned(),
            api_key: "sk-invalid-key".to_owned(),
            model: String::new(),
            dims: None,
            dir: String::new(),
        };
        let response = super::dispatch(
            &state,
            zk_contract_ipc::commands::SETTINGS_SET,
            &serde_json::json!({"settings": invalid}),
        );
        assert!(!response.ok, "failed embedding config must be rejected");
        assert!(
            response
                .error
                .as_deref()
                .is_some_and(|error| error.contains("embedding")),
            "{:?}",
            response.error
        );
        // The store still holds the last successfully saved settings.
        let after = state.runtime().settings().load().unwrap();
        assert_eq!(
            after.embedding,
            zk_contract_config::EmbeddingSettings::default()
        );
        std::env::remove_var("ZEOKEEL_HOME");
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn settings_set_persists_and_publishes_web_search_endpoint() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-search-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", &home);
        let state = super::AppState::new(zk_composed::build().expect("composed build"));
        zk_contract_config::clear_runtime_settings().unwrap();

        let settings = zk_contract_config::AppSettings {
            web_search: zk_contract_config::WebSearchSettings {
                base_url: "https://search.example.invalid/api".to_owned(),
                timeout_ms: 30_000,
            },
            ..zk_contract_config::AppSettings::default()
        };
        let saved = super::dispatch(
            &state,
            zk_contract_ipc::commands::SETTINGS_SET,
            &serde_json::json!({"settings": settings}),
        );
        assert!(saved.ok, "{:?}", saved.error);

        // Persisted (survives a restart: the store reloads the same section).
        let reloaded = state.runtime().settings().load().unwrap();
        assert_eq!(
            reloaded.web_search,
            zk_contract_config::WebSearchSettings {
                base_url: "https://search.example.invalid/api".to_owned(),
                timeout_ms: 30_000,
            }
        );

        // Published non-secret snapshot: tool-web-search reads settings > env.
        let published = zk_contract_config::runtime_settings().unwrap().unwrap();
        let published_search = published.web_search.expect("web_search published");
        assert_eq!(
            published_search.base_url,
            "https://search.example.invalid/api"
        );
        assert_eq!(published_search.timeout_ms, 30_000);

        // `settings.get` masks credentials but keeps the (keyless) search section.
        let got = super::dispatch(
            &state,
            zk_contract_ipc::commands::SETTINGS_GET,
            &serde_json::json!({}),
        );
        assert!(got.ok, "{:?}", got.error);
        let got_data = got.data.unwrap_or(serde_json::Value::Null);
        // settings.get returns `{ settings, revision }` (revision fence, spec R4).
        assert_eq!(got_data["revision"], serde_json::json!(1));
        assert_eq!(
            got_data["settings"]["web_search"]["base_url"],
            serde_json::Value::String("https://search.example.invalid/api".to_owned())
        );
        assert_eq!(
            got_data["settings"]["web_search"]["timeout_ms"],
            serde_json::json!(30_000)
        );
        assert_eq!(
            got_data["settings"]["provider"]["api_key"],
            serde_json::Value::String(String::new())
        );

        // Boot with a settings file publishes the saved web-search section too.
        zk_contract_config::clear_runtime_settings().unwrap();
        super::apply_persisted_settings(state.runtime()).unwrap();
        let booted = zk_contract_config::runtime_settings().unwrap().unwrap();
        assert_eq!(
            booted
                .web_search
                .expect("boot published web_search")
                .timeout_ms,
            30_000
        );

        // Invalid endpoint is rejected by settings.set and does not overwrite.
        let mut invalid = settings;
        invalid.web_search.base_url = "http://search.example.invalid".to_owned();
        let rejected = super::dispatch(
            &state,
            zk_contract_ipc::commands::SETTINGS_SET,
            &serde_json::json!({"settings": invalid}),
        );
        assert!(!rejected.ok, "plain-http search endpoint must be rejected");
        let unchanged = state.runtime().settings().load().unwrap();
        assert_eq!(
            unchanged.web_search.base_url,
            "https://search.example.invalid/api"
        );

        zk_contract_config::clear_runtime_settings().unwrap();
        std::env::remove_var("ZEOKEEL_HOME");
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn settings_set_revision_fence_rejects_stale_writes() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-revision-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", &home);
        let state = super::AppState::new(zk_composed::build().expect("composed build"));
        zk_contract_config::clear_runtime_settings().unwrap();

        // Fresh store starts at revision 0.
        let got = super::dispatch(
            &state,
            zk_contract_ipc::commands::SETTINGS_GET,
            &serde_json::json!({}),
        );
        assert!(got.ok, "{:?}", got.error);
        assert_eq!(
            got.data.unwrap_or(serde_json::Value::Null)["revision"],
            serde_json::json!(0)
        );

        // Fenced save with the current revision → accepted, revision 1.
        let dark = zk_contract_config::AppSettings {
            theme: "dark".to_owned(),
            ..zk_contract_config::AppSettings::default()
        };
        let saved = super::dispatch(
            &state,
            zk_contract_ipc::commands::SETTINGS_SET,
            &serde_json::json!({"settings": dark, "expected_revision": 0}),
        );
        assert!(saved.ok, "{:?}", saved.error);
        let saved_data = saved.data.unwrap_or(serde_json::Value::Null);
        assert_eq!(saved_data["revision"], serde_json::json!(1));
        assert_eq!(
            saved_data["settings"]["theme"],
            serde_json::Value::String("dark".to_owned())
        );

        // Stale fence → rejected with the stable code, nothing written.
        let light = zk_contract_config::AppSettings {
            theme: "light".to_owned(),
            ..zk_contract_config::AppSettings::default()
        };
        let rejected = super::dispatch(
            &state,
            zk_contract_ipc::commands::SETTINGS_SET,
            &serde_json::json!({"settings": light, "expected_revision": 0}),
        );
        assert!(!rejected.ok, "stale revision must be rejected");
        assert!(
            rejected
                .error
                .as_deref()
                .is_some_and(|error| error.starts_with("settings/revision-conflict")),
            "{:?}",
            rejected.error
        );
        assert_eq!(state.runtime().settings().load().unwrap().theme, "dark");
        assert_eq!(state.runtime().settings().revision().unwrap(), 1);

        // Retry with the current revision → accepted, revision 2.
        let retried = super::dispatch(
            &state,
            zk_contract_ipc::commands::SETTINGS_SET,
            &serde_json::json!({"settings": light, "expected_revision": 1}),
        );
        assert!(retried.ok, "{:?}", retried.error);
        assert_eq!(
            retried.data.unwrap_or(serde_json::Value::Null)["revision"],
            serde_json::json!(2)
        );

        // Legacy client (no expected_revision) keeps the old unfenced behaviour.
        let auto = zk_contract_config::AppSettings {
            theme: "auto".to_owned(),
            ..zk_contract_config::AppSettings::default()
        };
        let legacy = super::dispatch(
            &state,
            zk_contract_ipc::commands::SETTINGS_SET,
            &serde_json::json!({"settings": auto}),
        );
        assert!(legacy.ok, "{:?}", legacy.error);
        assert_eq!(
            legacy.data.unwrap_or(serde_json::Value::Null)["revision"],
            serde_json::json!(3)
        );

        // A malformed fence is a payload error (not a silent unfenced save).
        let bad = super::dispatch(
            &state,
            zk_contract_ipc::commands::SETTINGS_SET,
            &serde_json::json!({"settings": dark, "expected_revision": "one"}),
        );
        assert!(!bad.ok);
        assert_eq!(state.runtime().settings().revision().unwrap(), 3);

        zk_contract_config::clear_runtime_settings().unwrap();
        std::env::remove_var("ZEOKEEL_HOME");
        std::fs::remove_dir_all(&home).unwrap();
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
        let snapshot = super::runtime_snapshot_for(&tools_only, true, true, true);
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
        let _home_guard = HOME_LOCK.lock().unwrap();
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
        let files = super::list_workspace_files(&root, "", 2, 300).unwrap();
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

    use zk_contract_agent::HistoryRole;
    use zk_contract_session::Message as SessionMessage;

    fn replay_message(role: &str, content: &str, status: &str) -> SessionMessage {
        SessionMessage {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: "c".to_owned(),
            role: role.to_owned(),
            content: content.to_owned(),
            created_at_ms: 1,
            status: status.to_owned(),
            usage: None,
            turn_id: String::new(),
        }
    }

    #[test]
    fn replay_history_drops_current_prompt_and_maps_roles() {
        let messages: Vec<SessionMessage> = vec![
            replay_message("user", "earlier question", "done"),
            replay_message("assistant", "earlier answer", "done"),
            replay_message("user", "current prompt", "done"),
        ];
        let history = super::replay_history(&messages, "current prompt");
        assert_eq!(history.len(), 2, "current prompt is not replayed twice");
        assert_eq!(history[0].role, HistoryRole::User);
        assert_eq!(history[0].content, "earlier question");
        assert_eq!(history[1].role, HistoryRole::Assistant);
        assert_eq!(history[1].content, "earlier answer");
    }

    #[test]
    fn replay_history_skips_tool_rows_and_assistant_errors() {
        let messages: Vec<SessionMessage> = vec![
            replay_message("user", "q1", "done"),
            replay_message("tool", "{\"name\":\"fs.read\"}", "done"),
            replay_message("assistant", "错误：boom", "error"),
            replay_message("assistant", "ok", "done"),
        ];
        let history = super::replay_history(&messages, "next");
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].content, "q1");
        assert_eq!(history[1].content, "ok");
        assert!(history.iter().all(|item| item.content != "{}"));
    }

    #[test]
    fn replay_history_caps_the_tail() {
        let mut messages: Vec<SessionMessage> = (0..60)
            .map(|index| replay_message("user", &format!("message-{index}"), "done"))
            .collect();
        messages.push(replay_message("user", "latest prompt", "done"));
        let history = super::replay_history(&messages, "latest prompt");
        assert_eq!(history.len(), super::MAX_REPLAY_MESSAGES);
        assert_eq!(history.last().expect("last").content, "message-59");
        assert_eq!(
            history.first().expect("first").content,
            format!("message-{}", 60 - super::MAX_REPLAY_MESSAGES)
        );
    }

    #[test]
    fn connector_catalog_config_is_applied_and_secrets_rejected() {
        zk_connector_mcp::clear_catalog_config().unwrap();
        let catalog = serde_json::json!({"items": [{
            "id": "desk-test",
            "name": "Desktop MCP",
            "transport": "stdio",
            "enabled": true,
            "config": {"command": "python3", "args": ["server.py"], "timeout_ms": 1000}
        }]})
        .to_string();
        assert_eq!(super::apply_connector_catalog(&catalog).unwrap(), 1);
        assert_eq!(
            zk_connector_mcp::configured_server_ids().unwrap(),
            vec!["desk-test".to_owned()]
        );
        let secret = serde_json::json!({"items": [{
            "id": "bad",
            "name": "Bad MCP",
            "transport": "stdio",
            "enabled": true,
            "config": {"command": "x", "env": {"API_TOKEN": "v"}}
        }]})
        .to_string();
        assert!(super::apply_connector_catalog(&secret).is_err());
        zk_connector_mcp::clear_catalog_config().unwrap();
        assert!(zk_connector_mcp::configured_server_ids()
            .unwrap()
            .is_empty());
    }

    // ---- MCP connector OAuth 2.0 public-client + PKCE flow (docs/07 §6) ----

    fn oauth_test_env(home: &std::path::Path) {
        std::env::set_var("ZEOKEEL_HOME", home);
        std::env::set_var("ZEOKEEL_DEV_KEYCHAIN_FALLBACK", "1");
    }

    fn oauth_test_cleanup() {
        crate::mcp_oauth::set_test_opener(None);
        std::env::remove_var("ZEOKEEL_HOME");
        std::env::remove_var("ZEOKEEL_DEV_KEYCHAIN_FALLBACK");
    }

    fn oauth_payload(
        connector_id: &str,
        server: &crate::mcp_oauth::test_support::FakeOAuthServer,
    ) -> Value {
        let config = crate::mcp_oauth::test_support::config_for(server);
        json!({
            "connector_id": connector_id,
            "oauth": serde_json::to_value(&config).expect("oauth metadata"),
            "timeout_seconds": 60,
        })
    }

    fn wait_oauth_status(state: &std::sync::Arc<super::AppState>, connector_id: &str) -> Value {
        let started = std::time::Instant::now();
        loop {
            let response = super::dispatch(
                state,
                zk_contract_ipc::commands::OAUTH_STATUS,
                &json!({ "connector_id": connector_id }),
            );
            assert!(response.ok, "{:?}", response.error);
            let data = response.data.unwrap_or(Value::Null);
            let flow_state = data["state"].as_str().unwrap_or_default().to_owned();
            if flow_state != "pending" {
                return data;
            }
            assert!(
                started.elapsed() < std::time::Duration::from_secs(15),
                "oauth flow did not finish: {data}"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    #[test]
    fn mcp_oauth_ipc_authorize_status_and_keychain_roundtrip() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-oauth-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        oauth_test_env(&home);
        let server = crate::mcp_oauth::test_support::FakeOAuthServer::start(
            crate::mcp_oauth::test_support::FakeOAuthConfig::default(),
        );
        let opener: crate::mcp_oauth::TestOpener = std::sync::Arc::new(
            crate::mcp_oauth::test_support::spawning_opener(std::time::Duration::from_millis(200)),
        );
        crate::mcp_oauth::set_test_opener(Some(opener));
        let state = super::AppState::new(zk_composed::build().expect("composed build"));
        let payload = oauth_payload("mcp-demo", &server);

        // Unknown connector is idle.
        let idle = super::dispatch(
            &state,
            zk_contract_ipc::commands::OAUTH_STATUS,
            &json!({ "connector_id": "mcp-demo" }),
        );
        assert_eq!(idle.data.unwrap_or(Value::Null)["state"], "idle");

        let started = super::dispatch(&state, zk_contract_ipc::commands::OAUTH_AUTHORIZE, &payload);
        assert!(started.ok, "{:?}", started.error);
        assert_eq!(started.data.unwrap_or(Value::Null)["started"], true);

        // A second authorize while the browser flow is running is rejected, not duplicated.
        let duplicate =
            super::dispatch(&state, zk_contract_ipc::commands::OAUTH_AUTHORIZE, &payload);
        assert!(!duplicate.ok);
        assert_eq!(duplicate.error.as_deref(), Some("OAUTH_ALREADY_PENDING"));

        let status = wait_oauth_status(&state, "mcp-demo");
        assert_eq!(status["state"], "authorized", "{status}");
        assert!(status["error"].is_null());
        // The polling surface must never echo tokens.
        let status_text = status.to_string();
        assert!(!status_text.contains("access-"), "{status_text}");
        assert!(!status_text.contains("refresh-"), "{status_text}");

        // Tokens were saved through the existing keychain IPC (dev fallback file).
        let loaded = super::dispatch(
            &state,
            zk_contract_ipc::commands::OAUTH_TOKEN_LOAD,
            &json!({ "connector_id": "mcp-demo" }),
        );
        assert!(loaded.ok, "{:?}", loaded.error);
        let raw = loaded.data.unwrap_or(Value::Null)["tokens"]
            .as_str()
            .expect("stored token json string")
            .to_owned();
        let tokens = crate::mcp_oauth::OAuthTokens::from_stored_json(&raw).expect("token parse");
        assert!(tokens.access_token.starts_with("access-"));
        assert!(tokens.refresh_token.is_some());
        assert_eq!(tokens.scope.as_deref(), Some("mcp.read"));
        // The bearer token was handed to the MCP transport for this server.
        assert!(zk_connector_mcp::has_oauth_token("mcp-demo"));
        // RFC 8252 loopback redirect with an ephemeral port was used.
        let redirect = server.observed_redirect_uri().expect("redirect observed");
        assert!(redirect.starts_with("http://127.0.0.1:"), "{redirect}");
        assert!(redirect.ends_with("/callback"), "{redirect}");

        // Refresh rotates the stored token in the background.
        let before = tokens.access_token.clone();
        let refreshing =
            super::dispatch(&state, zk_contract_ipc::commands::OAUTH_REFRESH, &payload);
        assert!(refreshing.ok, "{:?}", refreshing.error);
        let refreshed = wait_oauth_status(&state, "mcp-demo");
        assert_eq!(refreshed["state"], "authorized");
        let loaded = super::dispatch(
            &state,
            zk_contract_ipc::commands::OAUTH_TOKEN_LOAD,
            &json!({ "connector_id": "mcp-demo" }),
        );
        let raw = loaded.data.unwrap_or(Value::Null)["tokens"]
            .as_str()
            .expect("stored token json string")
            .to_owned();
        let rotated = crate::mcp_oauth::OAuthTokens::from_stored_json(&raw).expect("token parse");
        assert_ne!(rotated.access_token, before, "refresh must update storage");
        assert_ne!(rotated.refresh_token, tokens.refresh_token, "rotation");

        // Clear removes it; the flow state is unaffected but no token remains.
        let cleared = super::dispatch(
            &state,
            zk_contract_ipc::commands::OAUTH_TOKEN_CLEAR,
            &json!({ "connector_id": "mcp-demo" }),
        );
        assert!(cleared.ok);
        let empty = super::dispatch(
            &state,
            zk_contract_ipc::commands::OAUTH_TOKEN_LOAD,
            &json!({ "connector_id": "mcp-demo" }),
        );
        assert!(empty.data.unwrap_or(Value::Null)["tokens"].is_null());
        // Disconnect also drops the in-memory MCP bearer token.
        assert!(!zk_connector_mcp::has_oauth_token("mcp-demo"));

        oauth_test_cleanup();
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn mcp_oauth_ipc_rejects_invalid_metadata_and_surfaces_failures_without_secrets() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-oauth-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        oauth_test_env(&home);
        let state = super::AppState::new(zk_composed::build().expect("composed build"));

        // Invalid endpoint metadata is rejected before any browser/listener work.
        let invalid = json!({
            "connector_id": "mcp-demo",
            "oauth": {
                "client_id": "c",
                "scopes": [],
                "redirect_uri": "",
                "authorize_url": "http://auth.example.invalid/authorize",
                "token_url": "https://auth.example.invalid/token"
            }
        });
        let rejected =
            super::dispatch(&state, zk_contract_ipc::commands::OAUTH_AUTHORIZE, &invalid);
        assert!(!rejected.ok);
        assert!(
            rejected
                .error
                .as_deref()
                .is_some_and(|error| error.starts_with("OAUTH_VALIDATION")),
            "{:?}",
            rejected.error
        );
        // Missing/bad connector ids never touch the keychain.
        let bad_id = super::dispatch(
            &state,
            zk_contract_ipc::commands::OAUTH_AUTHORIZE,
            &json!({ "connector_id": "../evil", "oauth": {} }),
        );
        assert!(!bad_id.ok);

        // A token-endpoint failure is reported as a status code only.
        let server = crate::mcp_oauth::test_support::FakeOAuthServer::start(
            crate::mcp_oauth::test_support::FakeOAuthConfig {
                token_status: 500,
                ..crate::mcp_oauth::test_support::FakeOAuthConfig::default()
            },
        );
        let opener: crate::mcp_oauth::TestOpener = std::sync::Arc::new(
            crate::mcp_oauth::test_support::spawning_opener(std::time::Duration::ZERO),
        );
        crate::mcp_oauth::set_test_opener(Some(opener));
        let started = super::dispatch(
            &state,
            zk_contract_ipc::commands::OAUTH_AUTHORIZE,
            &oauth_payload("mcp-broken", &server),
        );
        assert!(started.ok, "{:?}", started.error);
        let status = wait_oauth_status(&state, "mcp-broken");
        assert_eq!(status["state"], "failed", "{status}");
        let error = status["error"].as_str().unwrap_or_default();
        assert_eq!(error, "OAUTH_TOKEN_HTTP_500");
        assert!(!status.to_string().contains("SECRET"), "{status}");
        // Failed flow must not persist a partial token.
        let loaded = super::dispatch(
            &state,
            zk_contract_ipc::commands::OAUTH_TOKEN_LOAD,
            &json!({ "connector_id": "mcp-broken" }),
        );
        assert!(loaded.data.unwrap_or(Value::Null)["tokens"].is_null());

        // Refresh without stored tokens is an explicit error.
        let no_token = super::dispatch(
            &state,
            zk_contract_ipc::commands::OAUTH_REFRESH,
            &json!({
                "connector_id": "mcp-demo",
                "oauth": {
                    "client_id": "c",
                    "scopes": [],
                    "redirect_uri": "",
                    "authorize_url": "https://auth.example.invalid/authorize",
                    "token_url": "https://auth.example.invalid/token"
                }
            }),
        );
        assert!(!no_token.ok);
        assert_eq!(no_token.error.as_deref(), Some("OAUTH_NO_TOKEN"));

        oauth_test_cleanup();
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn settings_set_normalizes_legacy_approval_policy() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-approval-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", &home);
        let state = super::AppState::new(zk_composed::build().expect("composed build"));

        // Legacy `auto_approve_read_only=false` (never consumed before) maps to strict.
        let legacy = zk_contract_config::AppSettings {
            auto_approve_read_only: false,
            ..zk_contract_config::AppSettings::default()
        };
        let saved = super::dispatch(
            &state,
            zk_contract_ipc::commands::SETTINGS_SET,
            &serde_json::json!({"settings": legacy}),
        );
        assert!(saved.ok, "{:?}", saved.error);
        let loaded = state.runtime().settings().load().unwrap();
        assert_eq!(
            loaded.tools.approval_policy,
            Some(zk_contract_config::ApprovalPolicy::Strict)
        );
        assert!(!loaded.auto_approve_read_only);
        let snapshot = zk_contract_config::runtime_settings().unwrap().unwrap();
        assert_eq!(
            snapshot
                .tools
                .expect("tools published for a non-default policy")
                .effective_approval_policy(),
            zk_contract_config::ApprovalPolicy::Strict
        );

        // Explicitly choosing ask canonicalizes to "no policy" + legacy mirror true.
        let mut ask = loaded.clone();
        ask.tools.approval_policy = Some(zk_contract_config::ApprovalPolicy::Ask);
        ask.auto_approve_read_only = false;
        let saved = super::dispatch(
            &state,
            zk_contract_ipc::commands::SETTINGS_SET,
            &serde_json::json!({"settings": ask}),
        );
        assert!(saved.ok, "{:?}", saved.error);
        let loaded = state.runtime().settings().load().unwrap();
        assert_eq!(loaded.tools.approval_policy, None);
        assert!(loaded.auto_approve_read_only);
        assert_eq!(
            loaded.effective_approval_policy(),
            zk_contract_config::ApprovalPolicy::Ask
        );

        zk_contract_config::clear_runtime_settings().unwrap();
        zk_contract_agent::clear_default_agent_settings().unwrap();
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn boot_normalizes_legacy_approval_policy_when_file_exists() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-approval-boot-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", &home);
        let state = super::AppState::new(zk_composed::build().expect("composed build"));

        let legacy = zk_contract_config::AppSettings {
            auto_approve_read_only: false,
            ..zk_contract_config::AppSettings::default()
        };
        state.runtime().settings().save(&legacy).unwrap();
        assert!(super::settings_file_exists());
        super::apply_persisted_settings(state.runtime()).unwrap();
        let loaded = state.runtime().settings().load().unwrap();
        assert_eq!(
            loaded.tools.approval_policy,
            Some(zk_contract_config::ApprovalPolicy::Strict)
        );
        // Idempotent: a second boot changes nothing.
        super::apply_persisted_settings(state.runtime()).unwrap();
        assert_eq!(state.runtime().settings().load().unwrap(), loaded);

        zk_contract_config::clear_runtime_settings().unwrap();
        crate::automation::clear_settings_override();
        zk_contract_agent::clear_default_agent_settings().unwrap();
        std::fs::remove_dir_all(&home).unwrap();
    }

    fn fake_mcp_server(mode: Option<&str>) -> zk_contract_connector::McpServerConfig {
        let script = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("fake_mcp.py");
        let mut args = vec![script.to_string_lossy().into_owned()];
        if let Some(mode) = mode {
            args.push(mode.to_owned());
        }
        zk_contract_connector::McpServerConfig {
            id: "probe".to_owned(),
            name: "Probe".to_owned(),
            transport: zk_contract_connector::ConnectorTransport::Stdio,
            command: "python3".to_owned(),
            args,
            env: std::collections::BTreeMap::new(),
            url: String::new(),
            timeout_ms: 0,
            oauth_required: false,
        }
    }

    fn poll_probe(state: &std::sync::Arc<super::AppState>, test_id: &str) -> serde_json::Value {
        for _ in 0..100 {
            let response = super::dispatch(
                state,
                zk_contract_ipc::commands::MCP_TEST_STATUS,
                &serde_json::json!({"test_id": test_id}),
            );
            if response.ok {
                let data = response.data.unwrap_or(Value::Null);
                if data["state"] != serde_json::json!("pending") {
                    return data;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        serde_json::json!({"state": "pending"})
    }

    #[test]
    fn mcp_settings_apply_and_validate_through_settings_set() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-mcp-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", &home);
        let state = super::AppState::new(zk_composed::build().expect("composed build"));

        let mut settings = zk_contract_config::AppSettings::default();
        settings.mcp.servers = vec![fake_mcp_server(None)];
        let saved = super::dispatch(
            &state,
            zk_contract_ipc::commands::SETTINGS_SET,
            &serde_json::json!({"settings": settings}),
        );
        assert!(saved.ok, "{:?}", saved.error);
        let ids = zk_connector_mcp::configured_server_ids().unwrap();
        assert_eq!(ids, vec!["probe".to_owned()]);
        assert!(!home.join("nonexistent").exists());

        // Duplicate ids are rejected by the local caps and nothing is persisted.
        let mut duplicate = settings.clone();
        duplicate.mcp.servers.push(fake_mcp_server(None));
        let before = state.runtime().settings().load().unwrap();
        let rejected = super::dispatch(
            &state,
            zk_contract_ipc::commands::SETTINGS_SET,
            &serde_json::json!({"settings": duplicate}),
        );
        assert!(!rejected.ok);
        assert!(rejected
            .error
            .as_deref()
            .is_some_and(|error| error.contains("mcp/validation")));
        assert_eq!(state.runtime().settings().load().unwrap(), before);

        zk_connector_mcp::clear_local_servers().unwrap();
        zk_contract_config::clear_runtime_settings().unwrap();
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn mcp_test_two_phase_probe_ok_and_error_codes() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-mcp-probe-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", &home);
        let state = super::AppState::new(zk_composed::build().expect("composed build"));

        let started = super::dispatch(
            &state,
            zk_contract_ipc::commands::MCP_TEST,
            &serde_json::json!({"server": fake_mcp_server(None)}),
        );
        assert!(started.ok, "{:?}", started.error);
        let test_id = started.data.unwrap_or(Value::Null)["test_id"]
            .as_str()
            .expect("test_id")
            .to_owned();
        let status = poll_probe(&state, &test_id);
        assert_eq!(status["state"], serde_json::json!("ok"), "{status}");
        assert_eq!(status["tools"], serde_json::json!(["echo"]));

        // Invalid draft server: rejected before probing.
        let mut invalid = fake_mcp_server(None);
        invalid.id = "bad id".to_owned();
        let rejected = super::dispatch(
            &state,
            zk_contract_ipc::commands::MCP_TEST,
            &serde_json::json!({"server": invalid}),
        );
        assert!(rejected
            .error
            .as_deref()
            .is_some_and(|error| error.contains("mcp/validation")));
        // Unknown test id is a closed error.
        let unknown = super::dispatch(
            &state,
            zk_contract_ipc::commands::MCP_TEST_STATUS,
            &serde_json::json!({"test_id": "missing"}),
        );
        assert!(!unknown.ok);
        assert_eq!(
            unknown.error.as_deref(),
            Some("mcp/unsupported: unknown test_id")
        );

        zk_connector_mcp::clear_local_servers().unwrap();
        zk_contract_config::clear_runtime_settings().unwrap();
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn connectors_dispatch_uses_contract_commands_and_closed_errors() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home =
            std::env::temp_dir().join(format!("zk-connectors-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", &home);
        let state = super::AppState::new(zk_composed::build().expect("composed build"));

        // Missing/invalid catalog: closed `connectors/catalog-invalid` and the
        // previous (empty) config stays untouched.
        let missing = super::dispatch(
            &state,
            zk_contract_ipc::commands::CONNECTORS_CONFIGURE,
            &serde_json::json!({}),
        );
        assert!(!missing.ok);
        assert!(missing
            .error
            .as_deref()
            .is_some_and(|error| error.starts_with("connectors/catalog-invalid")));
        let invalid = super::dispatch(
            &state,
            zk_contract_ipc::commands::CONNECTORS_CONFIGURE,
            &serde_json::json!({"catalog": "not-json"}),
        );
        assert!(!invalid.ok);
        assert!(invalid
            .error
            .as_deref()
            .is_some_and(|error| error.starts_with("connectors/catalog-invalid")));

        // Empty catalog is a valid (configured=0) state.
        let configured = super::dispatch(
            &state,
            zk_contract_ipc::commands::CONNECTORS_CONFIGURE,
            &serde_json::json!({"catalog": {"items": []}}),
        );
        assert!(configured.ok, "{:?}", configured.error);
        assert_eq!(
            configured.data.unwrap_or(Value::Null)["configured"],
            serde_json::json!(0)
        );

        let reload = super::dispatch(
            &state,
            zk_contract_ipc::commands::CONNECTORS_RELOAD,
            &serde_json::json!({}),
        );
        assert!(reload.ok, "{:?}", reload.error);
        assert_eq!(
            reload.data.unwrap_or(Value::Null)["reloaded"],
            serde_json::json!(true)
        );

        let clear = super::dispatch(
            &state,
            zk_contract_ipc::commands::CONNECTORS_CLEAR,
            &serde_json::json!({}),
        );
        assert!(clear.ok, "{:?}", clear.error);
        assert_eq!(
            clear.data.unwrap_or(Value::Null)["cleared"],
            serde_json::json!(true)
        );

        zk_contract_config::clear_runtime_settings().unwrap();
        std::fs::remove_dir_all(&home).unwrap();
    }

    fn attachment_fixture(
        home: &std::path::Path,
    ) -> (
        std::sync::Arc<super::AppState>,
        String,
        String,
        std::path::PathBuf,
    ) {
        let ws_path = home.join("ws");
        std::fs::create_dir_all(&ws_path).unwrap();
        let state = super::AppState::new(zk_composed::build().expect("composed build"));
        let created = super::dispatch(
            &state,
            zk_contract_ipc::commands::WORKSPACE_CREATE,
            &json!({ "path": ws_path.to_string_lossy(), "title": "ws" }),
        );
        assert!(created.ok, "{:?}", created.error);
        let workspace_id = created.data.unwrap_or(Value::Null)["id"]
            .as_str()
            .expect("workspace id")
            .to_owned();
        let conversation = super::dispatch(
            &state,
            zk_contract_ipc::commands::SESSION_CREATE,
            &json!({ "workspace_id": workspace_id, "title": "conv" }),
        );
        assert!(conversation.ok, "{:?}", conversation.error);
        let conversation_id = conversation.data.unwrap_or(Value::Null)["id"]
            .as_str()
            .expect("conversation id")
            .to_owned();
        (state, workspace_id, conversation_id, ws_path)
    }

    fn attachment_payload(
        workspace_id: &str,
        conversation_id: &str,
        name: &str,
        mime: &str,
        bytes: &[u8],
    ) -> Value {
        json!({
            "workspace_id": workspace_id,
            "conversation_id": conversation_id,
            "name": name,
            "mime": mime,
            "data_base64": base64::engine::general_purpose::STANDARD.encode(bytes),
        })
    }

    const PNG_HEADER: &[u8] = b"\x89PNG\r\n\x1a\n";

    #[test]
    fn attachment_save_writes_inside_the_workspace_with_canonical_metadata() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-attach-ok-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", &home);
        let (state, workspace_id, conversation_id, ws_path) = attachment_fixture(&home);

        let payload = attachment_payload(
            &workspace_id,
            &conversation_id,
            "shot.png",
            "application/octet-stream",
            PNG_HEADER,
        );
        let saved = super::dispatch(
            &state,
            zk_contract_ipc::commands::WORKSPACE_ATTACHMENT_SAVE,
            &payload,
        );
        assert!(saved.ok, "{:?}", saved.error);
        let data = saved.data.unwrap_or(Value::Null);
        let path = data["path"].as_str().expect("path");
        assert!(
            path.starts_with(&format!(".zeokeel/attachments/{conversation_id}/")),
            "{path}"
        );
        assert!(path.ends_with("-shot.png"), "{path}");
        assert_eq!(data["name"], json!("shot.png"));
        assert_eq!(data["bytes"], json!(PNG_HEADER.len()));
        // The host MIME wins over the client hint.
        assert_eq!(data["mime"], json!("image/png"));
        assert_eq!(std::fs::read(ws_path.join(path)).unwrap(), PNG_HEADER);

        // A second upload of the same file never overwrites (uuid-prefixed).
        let again = super::dispatch(
            &state,
            zk_contract_ipc::commands::WORKSPACE_ATTACHMENT_SAVE,
            &payload,
        );
        assert!(again.ok, "{:?}", again.error);
        assert_ne!(again.data.unwrap_or(Value::Null)["path"], json!(path));

        std::env::remove_var("ZEOKEEL_HOME");
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn attachment_save_rejects_invalid_files_without_writing() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-attach-bad-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", &home);
        let (state, workspace_id, conversation_id, ws_path) = attachment_fixture(&home);
        let save = |payload: &Value| {
            super::dispatch(
                &state,
                zk_contract_ipc::commands::WORKSPACE_ATTACHMENT_SAVE,
                payload,
            )
        };

        // Unsupported extension.
        let exe = save(&attachment_payload(
            &workspace_id,
            &conversation_id,
            "evil.exe",
            "application/octet-stream",
            b"MZ",
        ));
        assert!(!exe.ok);
        assert!(exe
            .error
            .unwrap_or_default()
            .starts_with("attachment/type-not-allowed"));

        // Empty file.
        let empty = save(&attachment_payload(
            &workspace_id,
            &conversation_id,
            "empty.txt",
            "text/plain",
            b"",
        ));
        assert!(!empty.ok);
        assert_eq!(empty.error.as_deref(), Some("attachment/empty"));

        // Magic mismatch (png extension, text bytes).
        let fake = save(&attachment_payload(
            &workspace_id,
            &conversation_id,
            "fake.png",
            "image/png",
            b"not a png",
        ));
        assert!(!fake.ok);
        assert!(fake
            .error
            .unwrap_or_default()
            .starts_with("attachment/invalid"));

        // Oversized (>1 MiB raw bytes).
        let mut oversized = vec![0_u8; super::MAX_ATTACHMENT_BYTES.saturating_add(1)];
        oversized[..PNG_HEADER.len()].copy_from_slice(PNG_HEADER);
        let large = save(&attachment_payload(
            &workspace_id,
            &conversation_id,
            "large.png",
            "image/png",
            &oversized,
        ));
        assert!(!large.ok);
        assert_eq!(large.error.as_deref(), Some("attachment/too-large"));

        // Invalid base64.
        let bad = save(&json!({
            "workspace_id": workspace_id,
            "conversation_id": conversation_id,
            "name": "x.txt",
            "mime": "text/plain",
            "data_base64": "@@@not-base64@@@",
        }));
        assert!(!bad.ok);
        assert!(bad
            .error
            .unwrap_or_default()
            .starts_with("attachment/invalid"));

        // Path separators / traversal in the declared name.
        for name in ["../escape.png", "a/b.png", "a\\b.png"] {
            let rejected = save(&attachment_payload(
                &workspace_id,
                &conversation_id,
                name,
                "image/png",
                PNG_HEADER,
            ));
            assert!(!rejected.ok, "{name}");
            assert!(
                rejected
                    .error
                    .unwrap_or_default()
                    .starts_with("attachment/invalid"),
                "{name}"
            );
        }

        assert!(
            !ws_path.join(".zeokeel").exists(),
            "rejected saves must not create the attachment directory"
        );
        std::env::remove_var("ZEOKEEL_HOME");
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn attachment_save_accepts_one_mib_boundary() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-attach-max-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", &home);
        let (state, workspace_id, conversation_id, ws_path) = attachment_fixture(&home);
        let mut exact = vec![b'a'; super::MAX_ATTACHMENT_BYTES];
        exact[..PNG_HEADER.len()].copy_from_slice(PNG_HEADER);
        let saved = super::dispatch(
            &state,
            zk_contract_ipc::commands::WORKSPACE_ATTACHMENT_SAVE,
            &attachment_payload(
                &workspace_id,
                &conversation_id,
                "exact.png",
                "image/png",
                &exact,
            ),
        );
        assert!(saved.ok, "{:?}", saved.error);
        let path = saved.data.unwrap_or(Value::Null)["path"]
            .as_str()
            .expect("path")
            .to_owned();
        assert_eq!(
            std::fs::read(ws_path.join(path)).unwrap().len(),
            super::MAX_ATTACHMENT_BYTES
        );
        std::env::remove_var("ZEOKEEL_HOME");
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn attachment_save_rejects_symlinked_attachments_directory() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-attach-link-{}", uuid::Uuid::new_v4()));
        let outside = std::env::temp_dir().join(format!("zk-attach-out-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::env::set_var("ZEOKEEL_HOME", &home);
        let (state, workspace_id, conversation_id, ws_path) = attachment_fixture(&home);
        std::os::unix::fs::symlink(&outside, ws_path.join(".zeokeel")).unwrap();

        let saved = super::dispatch(
            &state,
            zk_contract_ipc::commands::WORKSPACE_ATTACHMENT_SAVE,
            &attachment_payload(
                &workspace_id,
                &conversation_id,
                "shot.png",
                "image/png",
                PNG_HEADER,
            ),
        );
        assert!(!saved.ok);
        assert!(saved
            .error
            .unwrap_or_default()
            .starts_with("attachment/workspace-unavailable"));
        assert!(
            !outside.join("attachments").exists(),
            "symlink escape must not write outside the workspace"
        );
        std::env::remove_var("ZEOKEEL_HOME");
        std::fs::remove_dir_all(&home).unwrap();
        std::fs::remove_dir_all(&outside).unwrap();
    }

    #[test]
    fn attachment_save_enforces_conversation_workspace_ownership() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-attach-own-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", &home);
        let (state, workspace_id, _conversation_id, _ws_path) = attachment_fixture(&home);
        let other_ws = home.join("other");
        std::fs::create_dir_all(&other_ws).unwrap();
        let created = super::dispatch(
            &state,
            zk_contract_ipc::commands::WORKSPACE_CREATE,
            &json!({ "path": other_ws.to_string_lossy(), "title": "other" }),
        );
        assert!(created.ok, "{:?}", created.error);
        let other_workspace = created.data.unwrap_or(Value::Null)["id"]
            .as_str()
            .expect("other workspace id")
            .to_owned();
        let conversation = super::dispatch(
            &state,
            zk_contract_ipc::commands::SESSION_CREATE,
            &json!({ "workspace_id": other_workspace, "title": "other conv" }),
        );
        let other_conversation = conversation.data.unwrap_or(Value::Null)["id"]
            .as_str()
            .expect("other conversation id")
            .to_owned();

        // The conversation does not belong to the requested workspace.
        let rejected = super::dispatch(
            &state,
            zk_contract_ipc::commands::WORKSPACE_ATTACHMENT_SAVE,
            &attachment_payload(
                &workspace_id,
                &other_conversation,
                "shot.png",
                "image/png",
                PNG_HEADER,
            ),
        );
        assert!(!rejected.ok);
        assert_eq!(
            rejected.error.as_deref(),
            Some("session/workspace-mismatch")
        );
        std::env::remove_var("ZEOKEEL_HOME");
        std::fs::remove_dir_all(&home).unwrap();
    }

    /// Dispatches one command and returns its `data` (panicking on error; test-only).
    fn call_ok(state: &std::sync::Arc<super::AppState>, name: &str, payload: Value) -> Value {
        let response = super::dispatch(state, name, &payload);
        assert!(response.ok, "{name}: {:?}", response.error);
        response.data.unwrap_or(Value::Null)
    }

    fn phase2b_fixture(
        home: &std::path::Path,
    ) -> (
        std::sync::Arc<super::AppState>,
        String,
        String,
        std::path::PathBuf,
    ) {
        let ws_path = home.join("ws2b");
        std::fs::create_dir_all(ws_path.join("demo-data")).unwrap();
        std::fs::write(ws_path.join("demo-data/hello.txt"), "hello from phase2b\n").unwrap();
        std::fs::write(ws_path.join("demo-data/old.txt"), "old\n").unwrap();
        let state = super::AppState::new(zk_composed::build().expect("composed build"));
        let workspace = call_ok(
            &state,
            zk_contract_ipc::commands::WORKSPACE_CREATE,
            json!({
                "path": ws_path.to_string_lossy(),
                "title": "ws2b",
            }),
        );
        let workspace_id = workspace["id"].as_str().expect("workspace id").to_owned();
        let conversation = call_ok(
            &state,
            zk_contract_ipc::commands::SESSION_CREATE,
            json!({
                "workspace_id": workspace_id,
                "title": "conv",
            }),
        );
        let conversation_id = conversation["id"]
            .as_str()
            .expect("conversation id")
            .to_owned();
        (state, workspace_id, conversation_id, ws_path)
    }

    #[test]
    fn workspace_files_keeps_legacy_shape_and_supports_lazy_expansion() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join("zeokeel-test-phase2b-files");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", home.join("zeokeel-home"));
        let (state, workspace_id, _, _) = phase2b_fixture(&home);
        // Legacy no-argument call: depth≤2 bounded listing (nested file present).
        let legacy = call_ok(
            &state,
            zk_contract_ipc::commands::WORKSPACE_FILES,
            json!({
                "workspace_id": workspace_id,
            }),
        );
        let legacy_paths: Vec<String> = legacy["items"]
            .as_array()
            .expect("items")
            .iter()
            .filter_map(|entry| entry["path"].as_str().map(str::to_owned))
            .collect();
        assert!(legacy_paths
            .iter()
            .any(|path| path == "demo-data/hello.txt"));
        // Lazy expansion of one directory: direct children only.
        let lazy = call_ok(
            &state,
            zk_contract_ipc::commands::WORKSPACE_FILES,
            json!({
                "workspace_id": workspace_id,
                "path": "demo-data",
                "depth": 1,
                "limit": 200,
            }),
        );
        let lazy_paths: Vec<String> = lazy["items"]
            .as_array()
            .expect("items")
            .iter()
            .filter_map(|entry| entry["path"].as_str().map(str::to_owned))
            .collect();
        assert!(lazy_paths.iter().all(|path| path.starts_with("demo-data/")));
        assert_eq!(lazy_paths.len(), 2);
        // Traversal is rejected, never read.
        let escaped = super::dispatch(
            &state,
            zk_contract_ipc::commands::WORKSPACE_FILES,
            &json!({
                "workspace_id": workspace_id,
                "path": "../",
            }),
        );
        assert!(!escaped.ok);
        assert!(escaped
            .error
            .unwrap_or_default()
            .starts_with("workspace/path-escape"));
        std::env::remove_var("ZEOKEEL_HOME");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn workspace_file_read_previews_text_images_and_degrades_honestly() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join("zeokeel-test-phase2b-read");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", home.join("zeokeel-home"));
        let (state, workspace_id, _, ws_path) = phase2b_fixture(&home);
        let text = call_ok(
            &state,
            zk_contract_ipc::commands::WORKSPACE_FILE_READ,
            json!({
                "workspace_id": workspace_id,
                "path": "demo-data/hello.txt",
            }),
        );
        assert_eq!(text["kind"], "text");
        assert_eq!(text["encoding"], "utf8");
        assert_eq!(text["content"], "hello from phase2b\n");
        assert_eq!(text["truncated"], false);

        // Oversized text is truncated (not rejected) and flagged.
        let big = "x".repeat(300 * 1024);
        std::fs::write(ws_path.join("big.txt"), &big).unwrap();
        let huge = call_ok(
            &state,
            zk_contract_ipc::commands::WORKSPACE_FILE_READ,
            json!({
                "workspace_id": workspace_id,
                "path": "big.txt",
            }),
        );
        assert_eq!(huge["truncated"], true);
        assert_eq!(
            huge["content"].as_str().unwrap_or_default().len(),
            256 * 1024
        );

        // PNG image comes back base64-encoded without embedding CSP changes.
        std::fs::write(ws_path.join("shot.png"), [0x89_u8, 0x50, 0x4e, 0x47]).unwrap();
        let image = call_ok(
            &state,
            zk_contract_ipc::commands::WORKSPACE_FILE_READ,
            json!({
                "workspace_id": workspace_id,
                "path": "shot.png",
            }),
        );
        assert_eq!(image["kind"], "image");
        assert_eq!(image["encoding"], "base64");

        // PDF and binary files degrade with a readable reason.
        std::fs::write(ws_path.join("doc.pdf"), b"%PDF-1.4").unwrap();
        let pdf = call_ok(
            &state,
            zk_contract_ipc::commands::WORKSPACE_FILE_READ,
            json!({
                "workspace_id": workspace_id,
                "path": "doc.pdf",
            }),
        );
        assert_eq!(pdf["kind"], "unsupported");
        assert!(pdf["reason"]
            .as_str()
            .unwrap_or_default()
            .starts_with("file/unsupported"));

        // Symlink escape / traversal / missing file are rejected read-only.
        for path in ["../escape.txt", "/etc/passwd", "missing.txt"] {
            let response = super::dispatch(
                &state,
                zk_contract_ipc::commands::WORKSPACE_FILE_READ,
                &json!({
                    "workspace_id": workspace_id,
                    "path": path,
                }),
            );
            assert!(!response.ok, "{path} must fail");
        }
        std::env::remove_var("ZEOKEEL_HOME");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn terminal_runs_open_send_close_without_tool_rows() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join("zeokeel-test-phase2b-terminal");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", home.join("zeokeel-home"));
        let (state, workspace_id, conversation_id, _) = phase2b_fixture(&home);
        let opened = call_ok(
            &state,
            zk_contract_ipc::commands::TERMINAL,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "action": "open",
                "session": "e2e",
            }),
        );
        assert!(opened["sessions"]
            .as_array()
            .is_some_and(|list| !list.is_empty()));
        let sent = call_ok(
            &state,
            zk_contract_ipc::commands::TERMINAL,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "action": "send",
                "session": "e2e",
                "command": "printf term-ok",
                "timeout_ms": 5000,
            }),
        );
        assert!(sent["output"]
            .as_str()
            .unwrap_or_default()
            .contains("term-ok"));
        let huge = call_ok(
            &state,
            zk_contract_ipc::commands::TERMINAL,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "action": "send",
                "session": "e2e",
                "command": "head -c 100000 /dev/zero | tr '\\0' 'x'",
                "timeout_ms": 8000,
            }),
        );
        assert_eq!(
            huge["truncated"], true,
            "output over 64 KiB must be marked truncated"
        );
        let closed = call_ok(
            &state,
            zk_contract_ipc::commands::TERMINAL,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "action": "close",
                "session": "e2e",
            }),
        );
        assert!(!closed["sessions"]
            .as_array()
            .is_some_and(|list| list.iter().any(|id| id == "e2e")));
        // Wrong conversation is rejected with the C18 error code.
        let mismatched = super::dispatch(
            &state,
            zk_contract_ipc::commands::TERMINAL,
            &json!({
                "workspace_id": workspace_id,
                "conversation_id": "not-a-conversation",
                "action": "list",
            }),
        );
        assert!(!mismatched.ok);
        assert!(mismatched
            .error
            .unwrap_or_default()
            .starts_with("session/workspace-mismatch"));
        std::env::remove_var("ZEOKEEL_HOME");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn session_update_fork_and_search_respect_workspace_isolation() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join("zeokeel-test-phase2b-sessions");
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", home.join("zeokeel-home"));
        let (state, workspace_id, conversation_id, _) = phase2b_fixture(&home);
        // Seed two messages through the store (append is not an IPC surface).
        let sessions = state.runtime.sessions();
        for (id, role, content) in [
            ("m1", "user", "alpha needle content"),
            ("m2", "assistant", "beta reply"),
        ] {
            sessions
                .append_message(&zk_contract_session::Message {
                    id: id.to_owned(),
                    conversation_id: conversation_id.clone(),
                    role: role.to_owned(),
                    content: content.to_owned(),
                    created_at_ms: 1,
                    status: "done".to_owned(),
                    usage: None,
                    turn_id: String::new(),
                })
                .expect("append message");
        }
        let renamed = call_ok(
            &state,
            zk_contract_ipc::commands::SESSION_UPDATE,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "title": "renamed",
            }),
        );
        assert_eq!(renamed["title"], "renamed");
        let pinned = call_ok(
            &state,
            zk_contract_ipc::commands::SESSION_UPDATE,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "pinned": true,
            }),
        );
        assert_eq!(pinned["pinned"], true);
        let archived = call_ok(
            &state,
            zk_contract_ipc::commands::SESSION_UPDATE,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "archived": true,
            }),
        );
        assert_eq!(archived["archived"], true);
        // Empty title is rejected (session/invalid) and leaves the row unchanged.
        let empty = super::dispatch(
            &state,
            zk_contract_ipc::commands::SESSION_UPDATE,
            &json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "title": "   ",
            }),
        );
        assert!(!empty.ok);
        assert!(empty
            .error
            .unwrap_or_default()
            .starts_with("session/invalid"));

        let forked = call_ok(
            &state,
            zk_contract_ipc::commands::SESSION_FORK,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
            }),
        );
        let forked_id = forked["id"].as_str().expect("fork id").to_owned();
        assert_ne!(forked_id, conversation_id);
        assert_eq!(forked["workspace_id"], workspace_id);
        assert_eq!(forked["archived"], false);
        let copies = sessions
            .messages_in(&workspace_id, &forked_id)
            .expect("fork messages");
        assert_eq!(copies.len(), 2);
        assert!(copies
            .iter()
            .all(|message| message.content.contains("needle") || message.content.contains("beta")));

        // Search is case-insensitive, snippet-only, and current-workspace only.
        let hits = call_ok(
            &state,
            zk_contract_ipc::commands::SESSION_SEARCH,
            json!({
                "workspace_id": workspace_id,
                "query": "NEEDLE",
            }),
        );
        let items = hits["items"].as_array().expect("items");
        assert_eq!(items.len(), 2); // source + fork copy
        assert!(items.iter().any(|hit| hit["archived"] == true));
        assert!(items.iter().any(|hit| hit["archived"] == false));
        // Empty query short-circuits; over-long queries are rejected.
        let empty_search = call_ok(
            &state,
            zk_contract_ipc::commands::SESSION_SEARCH,
            json!({
                "workspace_id": workspace_id,
                "query": "   ",
            }),
        );
        assert_eq!(empty_search["items"].as_array().map(Vec::len), Some(0));
        let long_query = "x".repeat(201);
        let long = super::dispatch(
            &state,
            zk_contract_ipc::commands::SESSION_SEARCH,
            &json!({
                "workspace_id": workspace_id,
                "query": long_query,
            }),
        );
        assert!(!long.ok);
        std::env::remove_var("ZEOKEEL_HOME");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn command_surface_goal_override_export_feedback_round_trip() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-cmd-surface-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", home.join("zeokeel-home"));
        let (state, workspace_id, conversation_id, ws_path) = phase2b_fixture(&home);

        // Override: empty → set model/effort/policy → persisted file → partial clear.
        let initial = call_ok(
            &state,
            zk_contract_ipc::commands::SESSION_OVERRIDE_GET,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
            }),
        );
        assert_eq!(initial["overrides"]["is_session_override"], false);
        let set = call_ok(
            &state,
            zk_contract_ipc::commands::SESSION_OVERRIDE_SET,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "model": "  deepseek-chat  ",
                "reasoning_effort": "low",
                "approval_policy": "strict",
            }),
        );
        assert_eq!(set["overrides"]["model"], "deepseek-chat");
        assert_eq!(set["overrides"]["is_session_override"], true);
        assert!(ws_path
            .join(".zeokeel/conversation-overrides.json")
            .is_file());
        let invalid = super::dispatch(
            &state,
            zk_contract_ipc::commands::SESSION_OVERRIDE_SET,
            &json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "approval_policy": "sandbox",
            }),
        );
        assert!(!invalid.ok);

        // Goal: set → pause → resume → cancel (same `.zeokeel/goal.json` as tool.goal).
        let empty_goal = call_ok(
            &state,
            zk_contract_ipc::commands::GOAL_GET,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
            }),
        );
        assert!(empty_goal["goal"].is_null());
        let goal = call_ok(
            &state,
            zk_contract_ipc::commands::GOAL_SET,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "objective": "ship the command surface",
                "max_rounds": 3,
            }),
        );
        assert_eq!(goal["goal"]["status"], "active");
        assert_eq!(goal["goal"]["max_rounds"], 3);
        let paused = call_ok(
            &state,
            zk_contract_ipc::commands::GOAL_UPDATE,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "action": "pause",
            }),
        );
        assert_eq!(paused["goal"]["status"], "blocked");
        let resumed = call_ok(
            &state,
            zk_contract_ipc::commands::GOAL_UPDATE,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "action": "resume",
            }),
        );
        assert_eq!(resumed["goal"]["status"], "active");
        let cancelled = call_ok(
            &state,
            zk_contract_ipc::commands::GOAL_UPDATE,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "action": "cancel",
            }),
        );
        assert_eq!(cancelled["goal"]["status"], "cancelled");
        assert!(ws_path.join(".zeokeel/goal.json").is_file());

        // Export: needs messages; writes Markdown under `.zeokeel/exports/`.
        let empty_export = super::dispatch(
            &state,
            zk_contract_ipc::commands::SESSION_EXPORT,
            &json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "format": "markdown",
            }),
        );
        assert!(!empty_export.ok);
        state
            .runtime
            .sessions()
            .append_message(&zk_contract_session::Message {
                id: "m1".to_owned(),
                conversation_id: conversation_id.clone(),
                role: "user".to_owned(),
                content: "export me".to_owned(),
                created_at_ms: 1,
                status: "done".to_owned(),
                usage: None,
                turn_id: String::new(),
            })
            .unwrap();
        let exported = call_ok(
            &state,
            zk_contract_ipc::commands::SESSION_EXPORT,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "format": "markdown",
            }),
        );
        let export_path = exported["path"].as_str().expect("export path");
        assert!(export_path.contains("/.zeokeel/exports/"));
        let markdown = std::fs::read_to_string(export_path).unwrap();
        assert!(markdown.contains("export me"));
        assert_eq!(exported["mime"], "text/markdown");

        // Feedback: local JSON only, category/limit validation.
        let feedback = call_ok(
            &state,
            zk_contract_ipc::commands::FEEDBACK_SUBMIT,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "category": "idea",
                "description": "please keep the command menu fast",
            }),
        );
        assert_eq!(feedback["stored"], "local");
        let feedback_path = feedback["path"].as_str().expect("feedback path");
        assert!(feedback_path.contains("/.zeokeel/feedback/"));
        let bad = super::dispatch(
            &state,
            zk_contract_ipc::commands::FEEDBACK_SUBMIT,
            &json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "category": "spam",
            }),
        );
        assert!(!bad.ok);
        let too_long = "x".repeat(4097);
        let long = super::dispatch(
            &state,
            zk_contract_ipc::commands::FEEDBACK_SUBMIT,
            &json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "category": "bug",
                "description": too_long,
            }),
        );
        assert!(!long.ok);
        std::env::remove_var("ZEOKEEL_HOME");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn config_mutex_applies_override_then_restores_global() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-cmd-mutex-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", home.join("zeokeel-home"));
        let (state, workspace_id, conversation_id, _) = phase2b_fixture(&home);
        let guard = crate::command_surface::lock_config();
        let global = crate::command_surface::snapshot_settings(&*state.runtime.settings()).unwrap();
        // No override yet: apply is a no-op; restore keeps the global config.
        assert!(!crate::command_surface::apply_override(
            &state.runtime,
            &global,
            &workspace_id,
            &conversation_id
        )
        .unwrap());
        call_ok(
            &state,
            zk_contract_ipc::commands::SESSION_OVERRIDE_SET,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "model": "session-model",
                "reasoning_effort": "high",
                "approval_policy": "auto_read_only",
            }),
        );
        assert!(crate::command_surface::apply_override(
            &state.runtime,
            &global,
            &workspace_id,
            &conversation_id
        )
        .unwrap());
        crate::command_surface::restore_global(&state.runtime, &global).unwrap();
        drop(guard);
        std::env::remove_var("ZEOKEEL_HOME");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn compact_writes_marker_keeps_raw_history_and_is_idempotent() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-compact-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", home.join("zeokeel-home"));
        let (state, workspace_id, conversation_id, _) = phase2b_fixture(&home);
        let sessions = state.runtime().sessions();
        // 16 small raw messages (8 user/assistant pairs) => compactable prefix + kept window.
        for index in 0..16_u64 {
            let role = if index % 2 == 0 { "user" } else { "assistant" };
            sessions
                .append_message(&zk_contract_session::Message {
                    id: format!("m{index:02}"),
                    conversation_id: conversation_id.clone(),
                    role: role.to_owned(),
                    content: format!("message {index} about the plan"),
                    created_at_ms: index.saturating_add(1),
                    status: "done".to_owned(),
                    usage: None,
                    turn_id: format!("turn-{index}"),
                })
                .unwrap();
        }
        let before_raw = sessions
            .messages_in(&workspace_id, &conversation_id)
            .unwrap();
        let result = call_ok(
            &state,
            zk_contract_ipc::commands::SESSION_COMPACT,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
            }),
        );
        assert_eq!(result["noop"], false);
        let marker = &result["marker"];
        assert!(marker["marker_id"]
            .as_str()
            .unwrap_or_default()
            .starts_with("compaction:"));
        assert!(marker["compressed_messages"].as_u64().unwrap_or(0) >= 4);
        assert_eq!(marker["keep_recent"], 6);
        // Raw store keeps every original message (only the marker is appended).
        let after_raw = sessions
            .messages_in(&workspace_id, &conversation_id)
            .unwrap();
        assert_eq!(after_raw.len(), before_raw.len() + 1);
        assert_eq!(
            after_raw
                .iter()
                .filter(|message| message.id.starts_with("m"))
                .count(),
            before_raw.len()
        );
        // Search still reads raw history (including compacted-away messages).
        let hits = call_ok(
            &state,
            zk_contract_ipc::commands::SESSION_SEARCH,
            json!({
                "workspace_id": workspace_id,
                "query": "message 0",
            }),
        );
        assert!(hits["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty()));
        // Compacted view = marker + recent messages (through boundary).
        let view = call_ok(
            &state,
            zk_contract_ipc::commands::SESSION_COMPACTED_VIEW,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
            }),
        );
        let view_messages = view["messages"].as_array().expect("view messages");
        assert_eq!(view_messages[0]["role"], "system");
        assert!(view_messages[0]["id"]
            .as_str()
            .unwrap_or_default()
            .starts_with("compaction:"));
        assert!(view_messages.len() < after_raw.len());
        // Replay injects the summary as a user history message (not the marker JSON).
        let history = super::replay_history(&after_raw, "");
        assert!(history
            .iter()
            .any(|message| message.role == HistoryRole::User
                && message.content.contains("[Earlier conversation summary")));
        // Same boundary again: no-op, no second marker.
        let second = call_ok(
            &state,
            zk_contract_ipc::commands::SESSION_COMPACT,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
            }),
        );
        assert_eq!(second["noop"], true);
        let after_second = sessions
            .messages_in(&workspace_id, &conversation_id)
            .unwrap();
        assert_eq!(after_second.len(), after_raw.len());
        std::env::remove_var("ZEOKEEL_HOME");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn compact_rejects_streaming_and_oversized_input_without_writing() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-compact-err-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", home.join("zeokeel-home"));
        let (state, workspace_id, conversation_id, _) = phase2b_fixture(&home);
        let sessions = state.runtime().sessions();
        // Oversized input (mock extractive path still applies the 12k token cap).
        for index in 0..16_u64 {
            sessions
                .append_message(&zk_contract_session::Message {
                    id: format!("big{index:02}"),
                    conversation_id: conversation_id.clone(),
                    role: if index % 2 == 0 {
                        "user".to_owned()
                    } else {
                        "assistant".to_owned()
                    },
                    content: "x".repeat(6000),
                    created_at_ms: index.saturating_add(1),
                    status: "done".to_owned(),
                    usage: None,
                    turn_id: format!("turn-{index}"),
                })
                .unwrap();
        }
        let before = sessions
            .messages_in(&workspace_id, &conversation_id)
            .unwrap()
            .len();
        let oversized = super::dispatch(
            &state,
            zk_contract_ipc::commands::SESSION_COMPACT,
            &json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
            }),
        );
        assert!(!oversized.ok);
        assert!(oversized
            .error
            .unwrap_or_default()
            .starts_with("compact/too-large"));
        assert_eq!(
            sessions
                .messages_in(&workspace_id, &conversation_id)
                .unwrap()
                .len(),
            before
        );
        // Streaming conversation is rejected with compact/busy and writes nothing.
        state.mark_conversation_running(&conversation_id);
        let busy = super::dispatch(
            &state,
            zk_contract_ipc::commands::SESSION_COMPACT,
            &json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
            }),
        );
        state.unmark_conversation_running(&conversation_id);
        assert!(!busy.ok);
        assert!(busy.error.unwrap_or_default().starts_with("compact/busy"));
        assert_eq!(
            sessions
                .messages_in(&workspace_id, &conversation_id)
                .unwrap()
                .len(),
            before
        );
        std::env::remove_var("ZEOKEEL_HOME");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn plan_get_update_review_and_persistence() {
        let _home_guard = HOME_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!("zk-plan-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", home.join("zeokeel-home"));
        let (state, workspace_id, conversation_id, _) = phase2b_fixture(&home);
        let get = |state: &std::sync::Arc<crate::ipc::AppState>| {
            call_ok(
                state,
                zk_contract_ipc::commands::PLAN_GET,
                json!({
                    "workspace_id": workspace_id,
                    "conversation_id": conversation_id,
                }),
            )
        };
        // Never used: off/draft.
        let initial = get(&state);
        assert_eq!(initial["plan"]["mode"], "off");
        assert_eq!(initial["plan"]["status"], "draft");
        // Enter/off toggle persists.
        let entered = call_ok(
            &state,
            zk_contract_ipc::commands::PLAN_UPDATE,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "action": "enter",
            }),
        );
        assert_eq!(entered["plan"]["mode"], "active");
        assert_eq!(get(&state)["plan"]["mode"], "active");
        // No pending plan yet: review actions are readable errors.
        let no_pending = super::dispatch(
            &state,
            zk_contract_ipc::commands::PLAN_UPDATE,
            &json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "action": "approve",
            }),
        );
        assert!(!no_pending.ok);
        assert!(no_pending
            .error
            .unwrap_or_default()
            .starts_with("plan/not-active"));
        // Simulate the `exit_plan_mode` submission through the shared plan store.
        let root = super::workspace_root_of(&state, &workspace_id)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let mut file = zk_composed::zk_tool_plan::plans::load(&root).unwrap();
        file.submit(&conversation_id, "# Plan\n- step".to_owned(), 1)
            .unwrap();
        zk_composed::zk_tool_plan::plans::save(&root, &file).unwrap();
        assert_eq!(get(&state)["plan"]["status"], "pending_review");
        assert_eq!(get(&state)["plan"]["plan"], "# Plan\n- step");
        // Reject keeps plan mode active + feedback; continue returns to draft.
        let rejected = call_ok(
            &state,
            zk_contract_ipc::commands::PLAN_UPDATE,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "action": "reject",
                "feedback": "add tests",
            }),
        );
        assert_eq!(rejected["plan"]["status"], "rejected");
        assert_eq!(rejected["plan"]["mode"], "active");
        assert_eq!(rejected["plan"]["feedback"], "add tests");
        let continued = call_ok(
            &state,
            zk_contract_ipc::commands::PLAN_UPDATE,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "action": "continue",
            }),
        );
        assert_eq!(continued["plan"]["status"], "draft");
        assert_eq!(continued["plan"]["mode"], "active");
        // Second submission → approve (never executes; leaves plan mode).
        let mut file = zk_composed::zk_tool_plan::plans::load(&root).unwrap();
        file.submit(&conversation_id, "# Plan v2".to_owned(), 2)
            .unwrap();
        zk_composed::zk_tool_plan::plans::save(&root, &file).unwrap();
        let approved = call_ok(
            &state,
            zk_contract_ipc::commands::PLAN_UPDATE,
            json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "action": "approve",
            }),
        );
        assert_eq!(approved["plan"]["status"], "approved");
        assert_eq!(approved["plan"]["mode"], "off");
        assert_eq!(get(&state)["plan"]["status"], "approved");
        // Turn-start filter reads the persisted mode.
        let mut file = zk_composed::zk_tool_plan::plans::load(&root).unwrap();
        file.enter(&conversation_id, 3);
        zk_composed::zk_tool_plan::plans::save(&root, &file).unwrap();
        assert!(crate::plan::plan_mode_active(
            &state,
            &root,
            &conversation_id
        ));
        // Unknown action is a closed error; wrong workspace keeps C18 isolation.
        let invalid = super::dispatch(
            &state,
            zk_contract_ipc::commands::PLAN_UPDATE,
            &json!({
                "workspace_id": workspace_id,
                "conversation_id": conversation_id,
                "action": "explode",
            }),
        );
        assert!(!invalid.ok);
        let wrong = super::dispatch(
            &state,
            zk_contract_ipc::commands::PLAN_GET,
            &json!({
                "workspace_id": workspace_id,
                "conversation_id": "not-this-conversation",
            }),
        );
        assert!(!wrong.ok);
        assert!(wrong
            .error
            .unwrap_or_default()
            .starts_with("session/workspace-mismatch"));
        std::env::remove_var("ZEOKEEL_HOME");
        let _ = std::fs::remove_dir_all(&home);
    }
}
