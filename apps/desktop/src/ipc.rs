#![forbid(unsafe_code)]
//! Shared command dispatch used by both the Tauri shell and the HTTP serve carrier.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tokio::sync::broadcast;
use zk_contract_config::{AppSettings, ProviderKind};
use zk_contract_ipc::{commands, CommandResponse, UiEvent};
use zk_contract_session::Message;
use zk_contract_tool::TurnContext;

use crate::approval::ApprovalBroker;
use crate::events::{emit_kind, now_ms, BridgeSink, EVENT_BUFFER};

/// Composed runtime + IPC/event state shared by all transports.
pub struct AppState {
    runtime: Arc<zk_composed::Runtime>,
    pub bus: broadcast::Sender<UiEvent>,
    seq: AtomicU64,
    streams: Mutex<HashMap<String, Arc<AtomicBool>>>,
    approvals: Arc<ApprovalBroker>,
}

impl AppState {
    /// Build state around an already composed runtime.
    pub fn new(runtime: zk_composed::Runtime) -> Arc<Self> {
        let (bus, _) = broadcast::channel(EVENT_BUFFER);
        let approvals = ApprovalBroker::new();
        let concrete = Arc::clone(&approvals);
        let gate: Arc<dyn zk_contract_tool::ApprovalGate> = concrete;
        zk_kit_approval::set_global_gate(gate);
        Arc::new(Self {
            runtime: Arc::new(runtime),
            bus,
            seq: AtomicU64::new(1),
            streams: Mutex::new(HashMap::new()),
            approvals,
        })
    }

    /// Runtime accessor (tests).
    pub fn runtime(&self) -> &Arc<zk_composed::Runtime> {
        &self.runtime
    }

    fn seq(&self) -> &AtomicU64 {
        &self.seq
    }
}

/// Dispatch one command. Never panics; all failures become `CommandResponse::err`.
pub fn dispatch(state: &Arc<AppState>, name: &str, payload: &Value) -> CommandResponse {
    match name {
        commands::META_GET => CommandResponse::ok(json!({
            "edition": zk_composed::EDITION,
            "packages": zk_composed::PACKAGES,
            "version": env!("CARGO_PKG_VERSION"),
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
                    None => return CommandResponse::err("workspace/path-unavailable: home directory unavailable"),
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
            match state.runtime.sessions().rename_workspace(workspace_id, title) {
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
            let confirm = payload.get("confirm").and_then(Value::as_bool).unwrap_or(false);
            match state.runtime.sessions().remove_workspace(workspace_id, reassign, confirm) {
                Ok(result) => CommandResponse::ok(json!(result)),
                Err(error) => session_err(error),
            }
        }
        commands::SESSION_CREATE => {
            let title = payload.get("title").and_then(Value::as_str).unwrap_or("新会话");
            let Some(workspace_id) = payload.get("workspace_id").and_then(Value::as_str) else {
                return CommandResponse::err("session/workspace-required: missing workspace_id");
            };
            match state.runtime.sessions().create_conversation_in(workspace_id, title.trim()) {
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
            match state.runtime.sessions().messages_in(workspace_id, conversation_id) {
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
                Ok(workspaces) => match workspaces.iter().find(|workspace| workspace.id == workspace_id) {
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
            zk_composed::configure_llm(base_url, access_token, model);
            CommandResponse::ok(json!({ "configured": true }))
        }
        commands::UPDATE_CHECK => {
            let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
            match zk_composed::check_update(env!("CARGO_PKG_VERSION"), &platform) {
                Ok(release) => CommandResponse::ok(json!({ "release": release, "platform": platform })),
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
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut children: Vec<_> = entries.filter_map(Result::ok).collect();
        children.sort_by_key(|entry| entry.file_name());
        for entry in children {
            if items.len() >= MAX_ENTRIES {
                break;
            }
            let path = entry.path();
            let rel = path.strip_prefix(root).map(|value| value.to_string_lossy().replace('\\', "/")).unwrap_or_default();
            if rel.is_empty() || rel.starts_with(".git") || rel.contains("/.git/") {
                continue;
            }
            let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
            items.push(json!({ "path": rel, "depth": depth, "kind": if is_dir { "dir" } else { "file" } }));
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
    let Some(app_id) = payload.get("app_id").and_then(Value::as_str) else { return CommandResponse::err("missing app_id"); };
    if app_id.is_empty() || app_id.len() > 64 || !app_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') { return CommandResponse::err("APP_BAD_REQUEST"); }
    let Some((base_url, token)) = zk_composed::gateway_credentials() else { return CommandResponse::err("AUTH_REQUIRED"); };
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
    let url = format!("{}/api/client/v1/apps/{app_id}/request", base_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::new();
    match client.post(url).bearer_auth(token).json(&request).send() {
        Ok(response) => match response.error_for_status() {
            Ok(ok) => match ok.json::<Value>() { Ok(value) => CommandResponse::ok(value), Err(error) => CommandResponse::err(format!("APP_BAD_RESPONSE: {error}")) },
            Err(error) => CommandResponse::err(format!("APP_UNAVAILABLE: {error}")),
        },
        Err(error) => CommandResponse::err(format!("APP_UNAVAILABLE: {error}")),
    }
}

fn skill_root() -> Result<std::path::PathBuf, String> {
    let base = match std::env::var("ZEOKEEL_HOME") {
        Ok(value) => std::path::PathBuf::from(value),
        Err(_) => dirs::home_dir().ok_or_else(|| "home directory unavailable".to_owned())?.join(".zeokeel"),
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
        && value.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
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
    let root = match skill_root() { Ok(root) => root, Err(error) => return CommandResponse::err(error) };
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
            if let Err(error) = std::fs::create_dir_all(&destination) { let _ = std::fs::remove_dir_all(&temp); return CommandResponse::err(error.to_string()); }
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
            if let Err(error) = std::fs::create_dir_all(parent) { let _ = std::fs::remove_dir_all(&temp); return CommandResponse::err(error.to_string()); }
        }
        if let Err(error) = std::fs::write(&destination, &content) {
            let _ = std::fs::remove_dir_all(&temp);
            return CommandResponse::err(error.to_string());
        }
        entries.push((relative.to_string_lossy().replace('\\', "/"), content));
    }
    let tree = zk_kit_content::tree_sha256(entries);
    if let Some(parent) = target.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) { let _ = std::fs::remove_dir_all(&temp); return CommandResponse::err(error.to_string()); }
    }
    if let Err(error) = std::fs::rename(&temp, &target) {
        let _ = std::fs::remove_dir_all(&temp);
        return CommandResponse::err(format!("skills install: {error}"));
    }
    CommandResponse::ok(json!({ "installed": true, "tree_sha256": tree, "path": target.to_string_lossy() }))
}

fn dev_oauth_fallback() -> bool {
    std::env::var("ZEOKEEL_DEV_KEYCHAIN_FALLBACK").map(|value| value == "1").unwrap_or(false)
}

fn dev_oauth_path() -> Result<std::path::PathBuf, String> {
    let base = match std::env::var("ZEOKEEL_HOME") {
        Ok(value) => std::path::PathBuf::from(value),
        Err(_) => dirs::home_dir().ok_or_else(|| "home directory unavailable".to_owned())?.join(".zeokeel"),
    };
    Ok(base.join("oauth-dev.json"))
}

fn read_dev_oauth() -> serde_json::Map<String, Value> {
    let Ok(path) = dev_oauth_path() else { return serde_json::Map::new() };
    let Ok(raw) = std::fs::read_to_string(path) else { return serde_json::Map::new() };
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
    std::fs::write(&path, Value::Object(map.clone()).to_string()).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn oauth_entry(connector_id: &str) -> Result<keyring::Entry, String> {
    if connector_id.is_empty() || connector_id.len() > 128 || !connector_id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) {
        return Err("OAUTH_VALIDATION".to_owned());
    }
    keyring::Entry::new("zeokeel-connector-oauth", connector_id).map_err(|error| format!("keychain: {error}"))
}

fn oauth_save(payload: &Value) -> CommandResponse {
    let (Some(connector_id), Some(tokens)) = (payload.get("connector_id").and_then(Value::as_str), payload.get("tokens")) else {
        return CommandResponse::err("missing connector_id/tokens");
    };
    if !tokens.is_object() { return CommandResponse::err("OAUTH_VALIDATION"); }
    let entry = match oauth_entry(connector_id) {
        Ok(entry) => entry,
        Err(_) if dev_oauth_fallback() => {
            let mut map = read_dev_oauth();
            map.insert(connector_id.to_owned(), tokens.clone());
            return match write_dev_oauth(&map) { Ok(()) => CommandResponse::ok(json!({ "saved": true, "fallback": true })), Err(error) => CommandResponse::err(format!("keychain fallback: {error}")) };
        }
        Err(error) => return CommandResponse::err(error),
    };
    match entry.set_password(&tokens.to_string()) {
        Ok(()) => CommandResponse::ok(json!({ "saved": true })),
        Err(_error) if dev_oauth_fallback() => {
            let mut map = read_dev_oauth();
            map.insert(connector_id.to_owned(), tokens.clone());
            match write_dev_oauth(&map) { Ok(()) => CommandResponse::ok(json!({ "saved": true, "fallback": true })), Err(error) => CommandResponse::err(format!("keychain fallback: {error}")) }
        }
        Err(error) => CommandResponse::err(format!("keychain: {error}")),
    }
}

fn oauth_load(payload: &Value) -> CommandResponse {
    let Some(connector_id) = payload.get("connector_id").and_then(Value::as_str) else { return CommandResponse::err("missing connector_id"); };
    let entry = match oauth_entry(connector_id) {
        Ok(entry) => entry,
        Err(_) if dev_oauth_fallback() => return CommandResponse::ok(json!({ "tokens": read_dev_oauth().get(connector_id).map(Value::to_string) })),
        Err(error) => return CommandResponse::err(error),
    };
    match entry.get_password() {
        Ok(raw) => CommandResponse::ok(json!({ "tokens": raw })),
        Err(_) if dev_oauth_fallback() => CommandResponse::ok(json!({ "tokens": read_dev_oauth().get(connector_id).map(Value::to_string) })),
        Err(keyring::Error::NoEntry) => CommandResponse::ok(json!({ "tokens": null })),
        Err(error) => CommandResponse::err(format!("keychain: {error}")),
    }
}

fn oauth_clear(payload: &Value) -> CommandResponse {
    let Some(connector_id) = payload.get("connector_id").and_then(Value::as_str) else { return CommandResponse::err("missing connector_id"); };
    let entry = match oauth_entry(connector_id) {
        Ok(entry) => entry,
        Err(_) if dev_oauth_fallback() => {
            let mut map = read_dev_oauth();
            map.remove(connector_id);
            return match write_dev_oauth(&map) { Ok(()) => CommandResponse::ok(json!({ "cleared": true, "fallback": true })), Err(error) => CommandResponse::err(format!("keychain fallback: {error}")) };
        }
        Err(error) => return CommandResponse::err(error),
    };
    match entry.delete_credential() {
        Ok(()) => CommandResponse::ok(json!({ "cleared": true })),
        Err(_) if dev_oauth_fallback() => {
            let mut map = read_dev_oauth();
            map.remove(connector_id);
            match write_dev_oauth(&map) { Ok(()) => CommandResponse::ok(json!({ "cleared": true, "fallback": true })), Err(error) => CommandResponse::err(format!("keychain fallback: {error}")) }
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
    let root = match skill_root() { Ok(root) => root, Err(error) => return CommandResponse::err(error) };
    let target = root.join(name);
    if target.exists() {
        if let Err(error) = std::fs::remove_dir_all(&target) {
            return CommandResponse::err(format!("skills remove: {error}"));
        }
    }
    CommandResponse::ok(json!({ "removed": true }))
}

fn installed_skills() -> CommandResponse {
    let root = match skill_root() { Ok(root) => root, Err(error) => return CommandResponse::err(error) };
    let mut items = Vec::new();
    if let Ok(names) = std::fs::read_dir(&root) {
        for name in names.filter_map(Result::ok) {
            if !name.file_type().map(|kind| kind.is_dir()).unwrap_or(false) { continue; }
            let name_value = name.file_name().to_string_lossy().to_string();
            if let Ok(versions) = std::fs::read_dir(name.path()) {
                for version in versions.filter_map(Result::ok) {
                    if version.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
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
        return CommandResponse::err(format!("missing conversation_id in workspace {workspace_id}"));
    };
    let workspace_id = workspace_id.to_owned();
    let conversation_id = conversation_id.to_owned();
    let list = match sessions.list_conversations_in(&workspace_id) {
        Ok(list) => list,
        Err(error) => return CommandResponse::err(error.to_string()),
    };
    if !list.iter().any(|conversation| conversation.id == conversation_id) {
        return CommandResponse::err(format!("session/workspace-mismatch: {conversation_id}"));
    }
    let workspaces = match sessions.list_workspaces() {
        Ok(list) => list,
        Err(error) => return CommandResponse::err(error.to_string()),
    };
    let Some(workspace) = workspaces.into_iter().find(|workspace| workspace.id == workspace_id) else {
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

    let state = Arc::clone(state);
    let text = text.to_owned();
    let conversation_id_for_thread = conversation_id.clone();
    let turn_id = stream_id.clone();
    std::thread::spawn(move || {
        let seq = Arc::new(AtomicU64::new(1));
        let sink: Arc<dyn zk_contract_agent::EventSink> = Arc::new(BridgeSink {
            bus: state.bus.clone(),
            stream_id: turn_id.clone(),
            seq: Arc::clone(&seq),
            cancelled: Arc::clone(&cancelled),
            sessions: state.runtime.sessions(),
            conversation_id: conversation_id_for_thread.clone(),
        });
        let context = TurnContext { conversation_id: conversation_id_for_thread.clone(), workspace_id: workspace_root.0.clone(), workspace_root: workspace_root.1 };
        let result = state.runtime.run_stream_in_shared(&context, &turn_id, &text, sink);
        match result {
            Ok(reply) => {
                let message = Message {
                    id: uuid::Uuid::new_v4().to_string(),
                    conversation_id: conversation_id_for_thread.clone(),
                    role: "assistant".to_owned(),
                    content: reply.text,
                    created_at_ms: now_ms(),
                    status: if cancelled.load(Ordering::SeqCst) { "cancelled" } else { "done" }.to_owned(),
                };
                let _ = state.runtime.sessions().append_message(&message);
            }
            Err(error) => {
                let message = Message {
                    id: uuid::Uuid::new_v4().to_string(),
                    conversation_id: conversation_id_for_thread,
                    role: "assistant".to_owned(),
                    content: format!("错误：{error}"),
                    created_at_ms: now_ms(),
                    status: "error".to_owned(),
                };
                let _ = state.runtime.sessions().append_message(&message);
            }
        }
        if let Ok(mut streams) = state.streams.lock() {
            streams.remove(&turn_id);
        }
    });

    CommandResponse::ok(json!({
        "stream_id": stream_id,
        "conversation_id": conversation_id,
        "workspace_id": workspace_id,
    }))
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
    emit_kind(&state.bus, stream_id, state.seq(), "agent", json!({ "type": "turn_cancelled" }));
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
            let mut options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            if let Some(mode) = mode { options = options.unix_permissions(*mode); }
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
    fn installs_valid_archive_and_rejects_hostile_entries() {
        let home = std::env::temp_dir().join(format!("zk-skill-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("ZEOKEEL_HOME", &home);

        let valid = zip_bytes(&[("SKILL.md", b"# demo\n", None), ("scripts/run.sh", b"echo ok\n", None)]);
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
        assert!(home.join("skills/demo-skill/1.0.0/SKILL.md").is_file(), "hostile remove must not delete installed skills");

        let removed = super::remove_skill(&serde_json::json!({ "name": "demo-skill" }));
        assert!(removed.ok);
        assert!(!home.join("skills/demo-skill").exists());
        std::fs::remove_dir_all(&home).unwrap();
    }
}
