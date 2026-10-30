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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use zk_contract_ipc::CommandResponse;

use crate::ipc::{dispatch, AppState};

const ENABLE_ENV: &str = "ZEOKEEL_ENABLE_SCHEDULE_AUTORUN";
const ORIGIN: &str = "tool-schedule";
/// Upper bound for a persisted prompt (matches the agent's practical turn input).
const MAX_PROMPT_BYTES: usize = 32 * 1024;
/// Period between full schedule scans (unchanged from the original executor).
const TICK_INTERVAL: Duration = Duration::from_secs(10);
/// Stop-flag poll granularity; bounds how long shutdown waits for the worker.
const STOP_POLL: Duration = Duration::from_millis(100);

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

/// Handle to the background schedule executor thread.
///
/// Dropping it (or calling [`Self::stop`]) signals the worker and joins it. The
/// thread checks the stop flag every [`STOP_POLL`], so the join is bounded by the
/// poll interval plus one scan; kernel task cancellation/shutdown therefore
/// cannot leak the thread.
#[must_use = "dropping the executor stops the schedule thread; hold it while the app runs"]
pub struct ScheduleExecutor {
    stop: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl ScheduleExecutor {
    fn idle() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            join: None,
        }
    }

    fn spawn_with<F>(stop: Arc<AtomicBool>, poll: Duration, period: Duration, tick: F) -> Self
    where
        F: FnMut() + Send + 'static,
    {
        let worker_stop = Arc::clone(&stop);
        let join = std::thread::spawn(move || run_loop(&worker_stop, poll, period, tick));
        Self {
            stop,
            join: Some(join),
        }
    }

    /// Signal the worker to stop and wait for it to exit (idempotent).
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for ScheduleExecutor {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Run `tick` every `period`, checking `stop` every `poll` iteration.
fn run_loop<F>(stop: &AtomicBool, poll: Duration, period: Duration, mut tick: F)
where
    F: FnMut(),
{
    let mut elapsed = Duration::ZERO;
    while !stop.load(Ordering::SeqCst) {
        std::thread::sleep(poll);
        elapsed = elapsed.saturating_add(poll);
        if elapsed < period {
            continue;
        }
        elapsed = Duration::ZERO;
        if stop.load(Ordering::SeqCst) {
            break;
        }
        tick();
    }
}

/// Start the background scheduler (no-op unless auto-run is explicitly enabled).
///
/// The returned executor must be held by the kernel task; when the task is
/// aborted/disposed its `Drop` stops and joins the std thread.
pub fn start(state: Arc<AppState>) -> ScheduleExecutor {
    if !autorun_enabled() {
        eprintln!("zk-desktop: schedule auto-run disabled (set {ENABLE_ENV}=1 to enable)");
        return ScheduleExecutor::idle();
    }
    let stop = Arc::new(AtomicBool::new(false));
    ScheduleExecutor::spawn_with(stop, STOP_POLL, TICK_INTERVAL, move || tick(&state))
}

/// One scan of all workspaces: run due, tool-created schedules.
fn tick(state: &Arc<AppState>) {
    let Some(Value::Array(workspaces)) = data(dispatch(state, "workspace.list", &json!({}))) else {
        return;
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
                state,
                "session.create",
                &json!({"workspace_id": id, "title": "Scheduled task"}),
            )) else {
                continue;
            };
            let Some(conversation_id) = conversation.get("id").and_then(Value::as_str) else {
                continue;
            };
            let _ = dispatch(
                state,
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
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
    use super::{valid_entry, ScheduleExecutor, STOP_POLL, TICK_INTERVAL};
    use serde_json::json;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

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

    #[test]
    fn stop_signal_stops_future_ticks_and_joins() {
        let stop = Arc::new(AtomicBool::new(false));
        let counter = Arc::new(AtomicUsize::new(0));
        let worker_counter = Arc::clone(&counter);
        let mut executor = ScheduleExecutor::spawn_with(
            Arc::clone(&stop),
            Duration::from_millis(5),
            Duration::from_millis(10),
            move || {
                worker_counter.fetch_add(1, Ordering::SeqCst);
            },
        );
        let deadline = Instant::now() + Duration::from_secs(2);
        while counter.load(Ordering::SeqCst) < 2 {
            assert!(Instant::now() < deadline, "worker did not tick");
            std::thread::sleep(Duration::from_millis(5));
        }
        executor.stop();
        let after_stop = counter.load(Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(
            counter.load(Ordering::SeqCst),
            after_stop,
            "worker kept ticking after stop"
        );
    }

    #[test]
    fn dropping_executor_stops_the_thread() {
        let stop = Arc::new(AtomicBool::new(false));
        let counter = Arc::new(AtomicUsize::new(0));
        let worker_counter = Arc::clone(&counter);
        {
            let _executor = ScheduleExecutor::spawn_with(
                Arc::clone(&stop),
                Duration::from_millis(5),
                Duration::from_millis(10),
                move || {
                    worker_counter.fetch_add(1, Ordering::SeqCst);
                },
            );
        }
        assert!(stop.load(Ordering::SeqCst), "drop must set the stop flag");
        let after_drop = counter.load(Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(
            counter.load(Ordering::SeqCst),
            after_drop,
            "worker kept ticking after drop"
        );
        // Keep the constants referenced so the poll/period contract is explicit.
        assert!(STOP_POLL < TICK_INTERVAL);
    }
}
