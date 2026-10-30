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

use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use zk_contract_ipc::CommandResponse;

use crate::ipc::AppState;

/// Single IPC entry point exposed to the web UI.
#[tauri::command]
fn ipc_call(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<AppState>>,
    name: String,
    payload: serde_json::Value,
) -> CommandResponse {
    if window.label().starts_with("zk-app-") {
        return CommandResponse::err("APP_FORBIDDEN");
    }
    ipc::dispatch(&state, &name, &payload)
}

fn valid_app_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 64 && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn open_app_window_impl(app: &tauri::AppHandle, app_id: &str, entry: &str) -> Result<(), String> {
    if !valid_app_id(app_id) { return Err("APP_BAD_REQUEST".to_owned()); }
    let app_id = app_id.to_owned();
    let entry = entry.to_owned();
    let label = format!("zk-app-{app_id}");
    if let Some(window) = app.get_webview_window(&label) {
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }
    if entry.is_empty() || entry.contains("..") || entry.contains(':') { return Err("APP_BAD_REQUEST".to_owned()); }
    let url: tauri::Url = format!("zk-app://localhost/{app_id}/{entry}").parse().map_err(|_| "APP_BAD_URL".to_owned())?;
    let data_dir = std::env::var("ZEOKEEL_HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir().join("zeokeel")).join("app-webviews").join(&app_id);
    let args = app_id.clone();
    WebviewWindowBuilder::new(app, &label, WebviewUrl::External(url))
        .title(format!("ZeoKeel App {app_id}"))
        .inner_size(900.0, 640.0)
        .incognito(true)
        .data_directory(data_dir)
        .on_navigation(move |url| url.scheme() == "zk-app" && url.path().starts_with(&format!("/{args}/")))
        .build()
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Open an isolated static-UI app window under `zk-app://<app_id>/`.
#[tauri::command]
async fn open_app_window(app: tauri::AppHandle, window: tauri::WebviewWindow, app_id: String, entry: String) -> Result<(), String> {
    if window.label().starts_with("zk-app-") { return Err("APP_FORBIDDEN".to_owned()); }
    open_app_window_impl(&app, &app_id, &entry)
}

/// App-window-only request command; main/webview windows are rejected by label.
#[tauri::command]
fn app_request(window: tauri::WebviewWindow, state: tauri::State<'_, Arc<AppState>>, payload: serde_json::Value) -> CommandResponse {
    if !window.label().starts_with("zk-app-") {
        return CommandResponse::err("APP_FORBIDDEN");
    }
    ipc::dispatch(&state, zk_contract_ipc::commands::APP_REQUEST, &payload)
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
        .register_uri_scheme_protocol("zk-app", |_ctx, request| {
            let uri = request.uri();
            let path = uri.path().trim_start_matches('/');
            let Some((app_id, asset)) = path.split_once('/') else {
                return http::Response::builder().status(400).body(b"bad app url".to_vec()).unwrap_or_default();
            };
            if !valid_app_id(app_id) || asset.contains("..") {
                return http::Response::builder().status(400).body(b"bad app path".to_vec()).unwrap_or_default();
            }
            let Some((base_url, token)) = zk_composed::gateway_credentials() else {
                return http::Response::builder().status(401).body(b"auth required".to_vec()).unwrap_or_default();
            };
            let url = format!("{}/api/client/v1/apps/{app_id}/assets/{asset}", base_url.trim_end_matches('/'));
            let client = reqwest::blocking::Client::new();
            let Ok(response) = client.get(url).bearer_auth(token).send() else {
                return http::Response::builder().status(502).body(b"app unavailable".to_vec()).unwrap_or_default();
            };
            if !response.status().is_success() {
                return http::Response::builder().status(response.status().as_u16()).body(b"app asset unavailable".to_vec()).unwrap_or_default();
            }
            let content_type = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|value| value.to_str().ok()).unwrap_or("text/html; charset=utf-8").to_owned();
            let mut body = response.bytes().map(|bytes| bytes.to_vec()).unwrap_or_default();
            if content_type.starts_with("text/html") {
                let csp = b"<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'self' zk-app:; script-src 'unsafe-inline' zk-app:; style-src 'unsafe-inline' zk-app:; img-src data: zk-app:; connect-src 'none'\">";
                let injection = b"<script>window.zkAppRequest=function(payload){return window.__TAURI_INTERNALS__.invoke('app_request',{payload:payload}).then(function(r){if(!r.ok){return Promise.reject(r.error);}return r.data;});};</script>";
                if let Some(position) = body.windows(5).position(|window| window.eq_ignore_ascii_case(b"</head")) {
                    body.splice(position..position, csp.iter().copied());
                    let after_csp = position.saturating_add(csp.len());
                    body.splice(after_csp..after_csp, injection.iter().copied());
                } else {
                    let mut wrapped = b"<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data:; connect-src 'none'\">".to_vec();
                    wrapped.extend_from_slice(injection);
                    wrapped.extend_from_slice(&body);
                    body = wrapped;
                }
            }
            http::Response::builder().status(200).header("content-type", content_type).body(body).unwrap_or_default()
        })
        .invoke_handler(tauri::generate_handler![ipc_call, open_app_window, app_request])
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
            if let Ok(app_id) = std::env::var("ZEOKEEL_OPEN_APP_ID") {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = open_app_window_impl(&handle, &app_id, "index.html") {
                        eprintln!("zk-desktop: open app window failed: {error}");
                    }
                });
            }
            Ok(())
        });
    builder
        .run(tauri::generate_context!())
        .map_err(|error| format!("failed to run window: {error}"))
}
