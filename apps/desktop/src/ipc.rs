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

use crate::events::{emit_kind, now_ms, BridgeSink, EVENT_BUFFER};

/// Composed runtime + IPC/event state shared by all transports.
pub struct AppState {
    runtime: Arc<zk_composed::Runtime>,
    pub bus: broadcast::Sender<UiEvent>,
    seq: AtomicU64,
    streams: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl AppState {
    /// Build state around an already composed runtime.
    pub fn new(runtime: zk_composed::Runtime) -> Arc<Self> {
        let (bus, _) = broadcast::channel(EVENT_BUFFER);
        Arc::new(Self {
            runtime: Arc::new(runtime),
            bus,
            seq: AtomicU64::new(1),
            streams: Mutex::new(HashMap::new()),
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
        commands::SESSION_CREATE => {
            let title = payload.get("title").and_then(Value::as_str).unwrap_or("新会话");
            match state.runtime.sessions().create_conversation(title.trim()) {
                Ok(conversation) => CommandResponse::ok(json!(conversation)),
                Err(error) => CommandResponse::err(error.to_string()),
            }
        }
        commands::SESSION_LIST => match state.runtime.sessions().list_conversations() {
            Ok(list) => CommandResponse::ok(json!(list)),
            Err(error) => CommandResponse::err(error.to_string()),
        },
        commands::SESSION_MESSAGES => {
            let Some(conversation_id) = payload.get("conversation_id").and_then(Value::as_str) else {
                return CommandResponse::err("missing conversation_id");
            };
            match state.runtime.sessions().messages(conversation_id) {
                Ok(messages) => CommandResponse::ok(json!(messages)),
                Err(error) => CommandResponse::err(error.to_string()),
            }
        }
        commands::SETTINGS_GET => match state.runtime.settings().load() {
            Ok(settings) => CommandResponse::ok(json!(settings)),
            Err(error) => CommandResponse::err(error.to_string()),
        },
        commands::SETTINGS_SET => {
            let Some(value) = payload.get("settings") else {
                return CommandResponse::err("missing settings");
            };
            let settings: AppSettings = match serde_json::from_value(value.clone()) {
                Ok(settings) => settings,
                Err(error) => return CommandResponse::err(format!("invalid settings: {error}")),
            };
            if let Err(error) = validate_settings(&settings) {
                return CommandResponse::err(error);
            }
            match state.runtime.settings().save(&settings) {
                Ok(()) => CommandResponse::ok(json!(settings)),
                Err(error) => CommandResponse::err(error.to_string()),
            }
        }
        commands::CHAT_SEND => chat_send(state, payload),
        commands::CHAT_CANCEL => chat_cancel(state, payload),
        commands::APPROVAL_DECIDE => {
            CommandResponse::err("approval flow lands in the next increment")
        }
        other => CommandResponse::err(format!("unknown command: {other}")),
    }
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
    let conversation_id = match payload.get("conversation_id").and_then(Value::as_str) {
        Some(id) if !id.is_empty() => id.to_owned(),
        _ => {
            let title: String = text.chars().take(20).collect();
            match sessions.create_conversation(&title) {
                Ok(conversation) => conversation.id,
                Err(error) => return CommandResponse::err(error.to_string()),
            }
        }
    };
    let user_message = Message {
        id: uuid::Uuid::new_v4().to_string(),
        conversation_id: conversation_id.clone(),
        role: "user".to_owned(),
        content: text.to_owned(),
        created_at_ms: now_ms(),
    };
    if let Err(error) = sessions.append_message(&user_message) {
        return CommandResponse::err(error.to_string());
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
        let sink = BridgeSink {
            bus: state.bus.clone(),
            stream_id: turn_id.clone(),
            seq: Arc::clone(&seq),
            cancelled: Arc::clone(&cancelled),
            sessions: state.runtime.sessions(),
            conversation_id: conversation_id_for_thread.clone(),
        };
        let result = state.runtime.run_stream(&turn_id, &text, &sink);
        match result {
            Ok(reply) => {
                let message = Message {
                    id: uuid::Uuid::new_v4().to_string(),
                    conversation_id: conversation_id_for_thread.clone(),
                    role: "assistant".to_owned(),
                    content: reply.text,
                    created_at_ms: now_ms(),
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
                };
                let _ = state.runtime.sessions().append_message(&message);
            }
        }
        if let Ok(mut streams) = state.streams.lock() {
            streams.remove(&turn_id);
        }
    });

    CommandResponse::ok(json!({ "stream_id": stream_id, "conversation_id": conversation_id.clone() }))
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
