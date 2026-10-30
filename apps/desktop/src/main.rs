//! `zk-desktop`: Tauri 2 shell + HTTP serve carrier for the ZeoKeel client.
//!
//! The shell exposes exactly one IPC entry point (`ipc_call`) and delegates to
//! the composed runtime. The same dispatch is reused by the serve carrier so
//! Playwright can exercise real UI flows headlessly.
#![forbid(unsafe_code)]

mod events;
mod ipc;
mod serve;

use std::sync::Arc;

use tauri::Emitter;
use zk_contract_ipc::CommandResponse;

use crate::ipc::AppState;

/// Single IPC entry point exposed to the web UI.
#[tauri::command]
fn ipc_call(
    state: tauri::State<'_, Arc<AppState>>,
    name: String,
    payload: serde_json::Value,
) -> CommandResponse {
    ipc::dispatch(&state, &name, &payload)
}

fn build_state() -> Result<Arc<AppState>, String> {
    let runtime = zk_composed::build()?;
    Ok(AppState::new(runtime))
}

fn main() {
    let serve_mode = std::env::args().any(|arg| arg == "--serve")
        || std::env::var("ZEOKEEL_SERVE").map(|v| v == "1").unwrap_or(false);

    let state = match build_state() {
        Ok(state) => state,
        Err(error) => {
            eprintln!("zk-desktop: failed to build composed runtime: {error}");
            eprintln!("zk-desktop: run `zk compose --edition personal --profile default --channel official` first");
            std::process::exit(1);
        }
    };

    if serve_mode {
        let addr = std::env::var("ZEOKEEL_SERVE_ADDR").unwrap_or_else(|_| "127.0.0.1:8788".to_owned());
        if let Err(error) = serve::run(state, &addr) {
            eprintln!("zk-desktop serve: {error}");
            std::process::exit(1);
        }
        return;
    }

    if let Err(error) = run_window(state) {
        eprintln!("zk-desktop: {error}");
        std::process::exit(1);
    }
}

fn run_window(state: Arc<AppState>) -> Result<(), String> {
    let bus = state.bus.clone();
    let builder = tauri::Builder::default()
        .manage(Arc::clone(&state))
        .invoke_handler(tauri::generate_handler![ipc_call])
        .setup(move |app| {
            let handle = app.handle().clone();
            let mut receiver = bus.subscribe();
            tauri::async_runtime::spawn(async move {
                loop {
                    match receiver.recv().await {
                        Ok(event) => {
                            let _ = handle.emit("zk://event", &event);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            });
            Ok(())
        });
    builder
        .run(tauri::generate_context!())
        .map_err(|error| format!("failed to run window: {error}"))
}
