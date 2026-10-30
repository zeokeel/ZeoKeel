//! `zk-desktop`: Tauri 2 shell for the ZeoKeel minimal demo.
//!
//! The main process exposes exactly one IPC entry point (`ipc_call`); the
//! business command table is data-driven and lives behind the composed
//! runtime (`zk_composed`). No logic is duplicated in the shell.
#![forbid(unsafe_code)]

/// Single IPC entry point exposed to the static web UI.
///
/// Supported commands:
/// - `demo.chat`: payload `{"text": "..."}` → agent turn result JSON.
/// - `demo.meta`: composed edition / package inventory.
#[tauri::command]
fn ipc_call(
    state: tauri::State<'_, zk_composed::Runtime>,
    name: String,
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    match name.as_str() {
        "demo.chat" => {
            let text = payload
                .get("text")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "missing text".to_owned())?;
            if text.trim().is_empty() {
                return Err("missing text".to_owned());
            }
            state.run(text)
        }
        "demo.meta" => Ok(serde_json::json!({
            "edition": zk_composed::EDITION,
            "packages": zk_composed::PACKAGES,
        })),
        unknown => Err(format!("unknown command: {unknown}")),
    }
}

fn main() {
    let runtime = match zk_composed::build() {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("zk-desktop: failed to build composed runtime: {err}");
            eprintln!("zk-desktop: run `zk compose --edition personal --profile default --channel official` first");
            std::process::exit(1);
        }
    };

    tauri::Builder::default()
        .manage(runtime)
        .invoke_handler(tauri::generate_handler![ipc_call])
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| {
            eprintln!("zk-desktop: failed to run window: {err}");
            std::process::exit(1);
        });
}
