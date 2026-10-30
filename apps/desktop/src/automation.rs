//! Workspace schedule executor.
//!
//! Auto-run is **off by default**: a scheduled prompt starts an agent turn with no
//! human in the loop, so the executor only starts when `ZEOKEEL_ENABLE_SCHEDULE_AUTORUN`
//! is explicitly `1`/`true`/`yes`, and it only runs entries created by the
//! `schedule` tool (`"origin": "tool-schedule"`) whose shape is valid and bounded.
//!
//! `interval_seconds` drives repetition; `cron` is validated by the tool at creation
//! time and is still advisory here (the executor does not interpret cron fields).
//! Entries are read from `<workspace>/.zeokeel/schedules.json`, which is
//! workspace-writable content: treat it as untrusted and keep the opt-in gate.
#![forbid(unsafe_code)]
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use zk_contract_ipc::CommandResponse;

use crate::ipc::{dispatch, AppState};

const ENABLE_ENV: &str = "ZEOKEEL_ENABLE_SCHEDULE_AUTORUN";
const ORIGIN: &str = "tool-schedule";
/// Upper bound for a persisted prompt (matches the agent's practical turn input).
const MAX_PROMPT_BYTES: usize = 32 * 1024;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| u64::try_from(value.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn data(response: CommandResponse) -> Option<Value> {
    if response.ok {
        response.data
    } else {
        None
    }
}

fn autorun_enabled() -> bool {
    std::env::var(ENABLE_ENV).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes"
        )
    })
}

/// Validate one persisted entry; returns `(prompt, interval_seconds)` when runnable.
fn valid_entry(item: &Value) -> Option<(String, u64)> {
    if item.get("origin").and_then(Value::as_str) != Some(ORIGIN) {
        return None;
    }
    let prompt = item.get("prompt").and_then(Value::as_str)?;
    if prompt.trim().is_empty() || prompt.len() > MAX_PROMPT_BYTES {
        return None;
    }
    let interval = item.get("interval_seconds").and_then(Value::as_u64)?;
    if interval < 60 {
        return None;
    }
    if let Some(cron) = item.get("cron").and_then(Value::as_str) {
        if !cron_is_plausible(cron) {
            return None;
        }
    }
    Some((prompt.to_owned(), interval))
}

/// Structural sanity check for a 5-field cron expression.
///
/// Full bounds validation happens in `tool-schedule`; the executor only refuses
/// obviously malformed expressions so untrusted workspace files cannot inject junk.
fn cron_is_plausible(expression: &str) -> bool {
    let fields: Vec<&str> = expression.split_whitespace().collect();
    fields.len() == 5
        && fields.iter().all(|field| {
            !field.is_empty()
                && field
                    .chars()
                    .all(|c| c.is_ascii_digit() || matches!(c, '*' | '-' | ',' | '/'))
        })
}

/// Start the background scheduler (no-op unless auto-run is explicitly enabled).
pub fn start(state: Arc<AppState>) {
    if !autorun_enabled() {
        eprintln!("zk-desktop: schedule auto-run disabled (set {ENABLE_ENV}=1 to enable)");
        return;
    }
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(10));
        let Some(Value::Array(workspaces)) = data(dispatch(&state, "workspace.list", &json!({})))
        else {
            continue;
        };
        for workspace in workspaces {
            let (Some(id), Some(path)) = (
                workspace.get("id").and_then(Value::as_str),
                workspace.get("path").and_then(Value::as_str),
            ) else {
                continue;
            };
            let file = std::path::Path::new(path).join(".zeokeel/schedules.json");
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            let Ok(mut schedules) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            let Some(items) = schedules.as_array_mut() else {
                continue;
            };
            let mut changed = false;
            for item in items.iter_mut() {
                let enabled = item
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let due = item
                    .get("next_run_ms")
                    .and_then(Value::as_u64)
                    .unwrap_or(u64::MAX);
                if !enabled || now_ms() < due {
                    continue;
                }
                let Some((prompt, interval)) = valid_entry(item) else {
                    continue;
                };
                let Some(conversation) = data(dispatch(
                    &state,
                    "session.create",
                    &json!({"workspace_id": id, "title": "Scheduled task"}),
                )) else {
                    continue;
                };
                let Some(conversation_id) = conversation.get("id").and_then(Value::as_str) else {
                    continue;
                };
                let _ = dispatch(
                    &state,
                    "chat.send",
                    &json!({"workspace_id": id, "conversation_id": conversation_id, "text": prompt}),
                );
                if let Some(object) = item.as_object_mut() {
                    object.insert(
                        "next_run_ms".to_owned(),
                        json!(now_ms().saturating_add(interval.saturating_mul(1000))),
                    );
                }
                changed = true;
            }
            if changed {
                if let Ok(text) = serde_json::to_string(&schedules) {
                    let _ = std::fs::write(&file, text);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
    use super::valid_entry;
    use serde_json::json;

    #[test]
    fn accepts_tool_created_entry() {
        let entry = json!({
            "origin": "tool-schedule",
            "prompt": "standup",
            "interval_seconds": 60,
            "cron": "0 9 * * *",
        });
        assert_eq!(valid_entry(&entry), Some(("standup".to_owned(), 60)));
    }

    #[test]
    fn rejects_untrusted_or_malformed_entries() {
        // Missing origin marker (e.g. a hand-written workspace file).
        assert!(valid_entry(&json!({"prompt":"x","interval_seconds":60})).is_none());
        // Empty prompt / too-short interval / malformed cron.
        assert!(
            valid_entry(&json!({"origin":"tool-schedule","prompt":"","interval_seconds":60}))
                .is_none()
        );
        assert!(
            valid_entry(&json!({"origin":"tool-schedule","prompt":"x","interval_seconds":10}))
                .is_none()
        );
        assert!(valid_entry(
            &json!({"origin":"tool-schedule","prompt":"x","interval_seconds":60,"cron":"bad"})
        )
        .is_none());
    }
}
