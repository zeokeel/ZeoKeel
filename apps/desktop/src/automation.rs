//! Workspace schedule executor: runs due `.zeokeel/schedules.json` prompts.
#![forbid(unsafe_code)]
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use zk_contract_ipc::CommandResponse;

use crate::ipc::{dispatch, AppState};

fn now_ms() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).map(|value| u64::try_from(value.as_millis()).unwrap_or(u64::MAX)).unwrap_or(0) }

fn data(response: CommandResponse) -> Option<Value> { if response.ok { response.data } else { None } }

/// Start the background scheduler (no-op if no schedule files exist).
pub fn start(state: Arc<AppState>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(10));
        let Some(Value::Array(workspaces)) = data(dispatch(&state, "workspace.list", &json!({}))) else { continue; };
        for workspace in workspaces {
            let (Some(id), Some(path)) = (workspace.get("id").and_then(Value::as_str), workspace.get("path").and_then(Value::as_str)) else { continue; };
            let file = std::path::Path::new(path).join(".zeokeel/schedules.json");
            let Ok(text) = std::fs::read_to_string(&file) else { continue; };
            let Ok(mut schedules) = serde_json::from_str::<Value>(&text) else { continue; };
            let Some(items) = schedules.as_array_mut() else { continue; };
            let mut changed = false;
            for item in items.iter_mut() {
                let enabled = item.get("enabled").and_then(Value::as_bool).unwrap_or(false);
                let due = item.get("next_run_ms").and_then(Value::as_u64).unwrap_or(u64::MAX);
                let Some(prompt) = item.get("prompt").and_then(Value::as_str).map(str::to_owned) else { continue; };
                if !enabled || now_ms() < due { continue; }
                let interval = item.get("interval_seconds").and_then(Value::as_u64).unwrap_or(3600).max(60);
                let Some(conversation) = data(dispatch(&state, "session.create", &json!({"workspace_id": id, "title": "Scheduled task"}))) else { continue; };
                let Some(conversation_id) = conversation.get("id").and_then(Value::as_str) else { continue; };
                let _ = dispatch(&state, "chat.send", &json!({"workspace_id": id, "conversation_id": conversation_id, "text": prompt}));
                if let Some(object) = item.as_object_mut() { object.insert("next_run_ms".to_owned(), json!(now_ms().saturating_add(interval.saturating_mul(1000)))); }
                changed = true;
            }
            if changed { if let Ok(text) = serde_json::to_string(&schedules) { let _ = std::fs::write(&file, text); } }
        }
    });
}
