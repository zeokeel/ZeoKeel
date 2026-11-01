//! `zk-desktop`: Tauri 2 shell + HTTP serve carrier for the ZeoKeel client.
//!
//! The shell exposes exactly one IPC entry point (`ipc_call`) and delegates to
//! the composed runtime. The same dispatch is reused by the serve carrier so
//! Playwright can exercise real UI flows headlessly.
#![forbid(unsafe_code)]
// Windows release builds are GUI apps: without this the app opens a cmd window.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod approval;
mod automation;
mod changes;
mod command_surface;
mod compact;
mod events;
mod ipc;
mod mcp;
mod mcp_oauth;
mod native_client;
mod plan;
mod serve;

use std::sync::Arc;

use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use zk_contract_config::ProviderKind;
use zk_contract_ipc::CommandResponse;

use crate::ipc::AppState;

/// Kernel service: composed runtime identity (name/version/edition/packages).
static COMPOSED_RUNTIME_SERVICE: zk_bootstrap::ServiceKey<ComposedRuntimeIdentity> =
    zk_bootstrap::ServiceKey::new("zk.desktop.composed");
/// Kernel service: the composed desktop state (IPC/event/approval runtime).
static APP_STATE_SERVICE: zk_bootstrap::ServiceKey<AppState> =
    zk_bootstrap::ServiceKey::new("zk.desktop.state");

/// Identity of the composed desktop runtime, exposed as a kernel service.
#[derive(Clone, Debug)]
struct ComposedRuntimeIdentity {
    version: &'static str,
    edition: &'static str,
    package_count: usize,
}

/// Plugin that registers the composed runtime identity in the kernel (ADR-009/064).
struct ComposedRuntimePlugin {
    identity: ComposedRuntimeIdentity,
}

impl zk_bootstrap::Plugin for ComposedRuntimePlugin {
    fn name(&self) -> &'static str {
        "zk.desktop.composed"
    }

    fn apply(
        &self,
        ctx: zk_bootstrap::Context,
        _config: serde_json::Value,
    ) -> zk_bootstrap::CordisResult<()> {
        ctx.provide(COMPOSED_RUNTIME_SERVICE, Arc::new(self.identity.clone()))?;
        Ok(())
    }
}

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
    !value.is_empty()
        && value.len() <= 64
        && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn open_app_window_impl(app: &tauri::AppHandle, app_id: &str, entry: &str) -> Result<(), String> {
    if !valid_app_id(app_id) {
        return Err("APP_BAD_REQUEST".to_owned());
    }
    let app_id = app_id.to_owned();
    let entry = entry.to_owned();
    let label = format!("zk-app-{app_id}");
    if let Some(window) = app.get_webview_window(&label) {
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(());
    }
    if entry.is_empty() || entry.contains("..") || entry.contains(':') {
        return Err("APP_BAD_REQUEST".to_owned());
    }
    let url: tauri::Url = format!("zk-app://localhost/{app_id}/{entry}")
        .parse()
        .map_err(|_| "APP_BAD_URL".to_owned())?;
    let data_dir = std::env::var("ZEOKEEL_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("zeokeel"))
        .join("app-webviews")
        .join(&app_id);
    let args = app_id.clone();
    WebviewWindowBuilder::new(app, &label, WebviewUrl::CustomProtocol(url))
        .title(format!("ZeoKeel App {app_id}"))
        .inner_size(900.0, 640.0)
        .incognito(true)
        .data_directory(data_dir)
        .on_navigation(move |url| {
            url.scheme() == "zk-app" && url.path().starts_with(&format!("/{args}/"))
        })
        .build()
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Open an isolated static-UI app window under `zk-app://<app_id>/`.
#[tauri::command]
async fn open_app_window(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    app_id: String,
    entry: String,
) -> Result<(), String> {
    if window.label().starts_with("zk-app-") {
        return Err("APP_FORBIDDEN".to_owned());
    }
    open_app_window_impl(&app, &app_id, &entry)
}

/// App-window-only request command; main/webview windows are rejected by label.
#[tauri::command]
fn app_request(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<AppState>>,
    payload: serde_json::Value,
) -> CommandResponse {
    if !window.label().starts_with("zk-app-") {
        return CommandResponse::err("APP_FORBIDDEN");
    }
    ipc::dispatch(&state, zk_contract_ipc::commands::APP_REQUEST, &payload)
}

fn build_state() -> Result<
    (
        Arc<AppState>,
        zk_bootstrap::Bootstrap,
        tokio::runtime::Runtime,
    ),
    String,
> {
    // Kernel tasks need a host runtime that outlives the Tauri event loop.
    let task_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("start kernel runtime: {error}"))?;
    // Enter only while booting/registering tasks; Tauri/serve own the thread later.
    let (state, bootstrap) = {
        let _guard = task_runtime.enter();
        boot_desktop()?
    };
    Ok((state, bootstrap, task_runtime))
}

fn boot_desktop() -> Result<(Arc<AppState>, zk_bootstrap::Bootstrap), String> {
    // Kernel root first: the host owns every plugin effect and background task.
    let bootstrap =
        zk_bootstrap::boot("zk-desktop").map_err(|error| format!("boot kernel: {error}"))?;
    let runtime = zk_composed::build()?;
    // Personal edition may configure local media/embedding providers from env; the
    // enterprise client never does (AI only through the gateway, C7/ADR-031).
    if zk_composed::EDITION == "personal" {
        zk_composed::configure_capabilities_from_env();
    }
    apply_persisted_provider(&runtime);
    let persisted_settings = ipc::apply_persisted_settings(&runtime);
    let state = AppState::new(runtime);
    if let Err(error) = ipc::configure_persisted_capabilities(state.runtime()) {
        eprintln!("zk-desktop: capability settings not applied: {error}");
        state.record_agent_config_error(error);
    }
    match persisted_settings {
        Err(error) => {
            eprintln!("zk-desktop: settings from persisted settings not applied: {error}");
            state.record_agent_config_error(error);
        }
        Ok(Some(tools)) => {
            if let Err(error) = state.apply_tool_settings(&tools) {
                eprintln!("zk-desktop: tool/approval settings not applied: {error}");
                state.record_agent_config_error(error);
            }
        }
        Ok(None) => {}
    }
    bootstrap
        .provide(APP_STATE_SERVICE, Arc::clone(&state))
        .map_err(|error| format!("register app state: {error}"))?;
    let identity = ComposedRuntimeIdentity {
        version: env!("CARGO_PKG_VERSION"),
        edition: zk_composed::EDITION,
        package_count: zk_composed::PACKAGES.len(),
    };
    eprintln!(
        "zk-desktop: composed runtime edition={} version={} packages={}",
        identity.edition, identity.version, identity.package_count
    );
    bootstrap
        .register_plugin(
            Arc::new(ComposedRuntimePlugin { identity }),
            serde_json::json!({ "edition": zk_composed::EDITION }),
        )
        .map_err(|error| format!("register composed runtime: {error}"))?;
    // Schedule executor runs as a kernel-owned task (abort-on-dispose). The task
    // holds the executor handle, so kernel cancellation/shutdown drops it, which
    // signals the worker and joins the std thread (bounded by its stop poll).
    let schedule_state = Arc::clone(&state);
    bootstrap
        .spawn_task("zk.desktop.schedule", async move {
            let _executor = automation::start(schedule_state);
            std::future::pending::<()>().await;
        })
        .map_err(|error| format!("start schedule executor: {error}"))?;
    Ok((state, bootstrap))
}

/// Apply the provider persisted by a previous run (settings hot-swap, ADR-064).
///
/// The composed glue builds the model from env first; a saved provider wins. All
/// failures are startup diagnostics: the app must still boot so the user can fix
/// the provider in settings (where `settings.set` reports the error to the UI).
fn apply_persisted_provider(runtime: &zk_composed::Runtime) {
    // Enterprise builds take the gateway provider from the login response
    // (`auth.configure`); local settings must never override it (docs/04 §6: no BYOK).
    if zk_composed::EDITION != "personal" {
        return;
    }
    let settings = match runtime.settings().load() {
        Ok(settings) => settings,
        Err(error) => {
            eprintln!("zk-desktop: settings load failed: {error}");
            return;
        }
    };
    let provider = settings.provider;
    let kind = match provider.kind {
        ProviderKind::Mock => {
            eprintln!(
                "zk-desktop: no provider configured in settings; using env fallback until configured"
            );
            return;
        }
        ProviderKind::OpenAiCompatible => "openai",
        ProviderKind::Ollama => "ollama",
    };
    if provider.base_url.trim().is_empty() || provider.model.trim().is_empty() {
        eprintln!(
            "zk-desktop: persisted provider is incomplete (base_url/model empty); keeping env fallback"
        );
        return;
    }
    if let Err(error) =
        runtime.configure_llm(&provider.base_url, &provider.api_key, &provider.model, kind)
    {
        eprintln!("zk-desktop: provider from settings not applied: {error}");
    }
}

fn main() {
    native_client::install_client_header_env();
    let serve_mode = std::env::args().any(|arg| arg == "--serve")
        || std::env::var("ZEOKEEL_SERVE")
            .map(|v| v == "1")
            .unwrap_or(false);

    let (state, bootstrap, _task_runtime) = match build_state() {
        Ok(built) => built,
        Err(error) => {
            eprintln!("zk-desktop: failed to build composed runtime: {error}");
            eprintln!(
                "zk-desktop: run `zk compose --edition personal --profile default --channel official` first"
            );
            std::process::exit(1);
        }
    };
    let bootstrap = Arc::new(bootstrap);

    if serve_mode {
        let addr =
            std::env::var("ZEOKEEL_SERVE_ADDR").unwrap_or_else(|_| "127.0.0.1:8788".to_owned());
        let result = serve::run(state, &addr);
        if let Err(error) = bootstrap.shutdown() {
            eprintln!("zk-desktop: bootstrap shutdown failed: {error}");
        }
        if let Err(error) = result {
            eprintln!("zk-desktop serve: {error}");
            std::process::exit(1);
        }
        return;
    }

    if let Err(error) = run_window(state, Arc::clone(&bootstrap)) {
        eprintln!("zk-desktop: {error}");
        std::process::exit(1);
    }
}

fn run_window(state: Arc<AppState>, bootstrap: Arc<zk_bootstrap::Bootstrap>) -> Result<(), String> {
    let bus = state.bus.clone();
    let setup_bootstrap_events = Arc::clone(&bootstrap);
    let setup_bootstrap_open = Arc::clone(&bootstrap);
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
            if std::env::var("ZEOKEEL_APP_SELFTEST").ok().as_deref() == Some("1") && app_id == "smoke-app" {
                let html = r#"<!doctype html><html><body><h1>app selftest</h1><pre id=out>running</pre><script>
document.title='SELFTEST|start';
(async()=>{
function show(v){return JSON.stringify(v).slice(0,160);}
async function call(name,args){try{return show(await window.__TAURI_INTERNALS__.invoke(name,args));}catch(e){return 'THROW:'+String(e);}}
const appRequest=await call('app_request',{payload:{app_id:'smoke-app',body_b64:''}});
const ipc=await call('ipc_call',{name:'status',payload:{}});
const open=await call('open_app_window',{appId:'other'});
const core=await call('plugin:event|listen',{event:'x',target:{kind:'Any'},handler:1});
document.getElementById('out').textContent=JSON.stringify({appRequest,ipc,open,core});
document.title='SELFTEST|'+JSON.stringify({appRequest,ipc,open,core});
})();
</script></body></html>"#;
                return http::Response::builder().status(200).header("content-type", "text/html; charset=utf-8").body(html.as_bytes().to_vec()).unwrap_or_default();
            }
            let Some((base_url, token)) = zk_composed::gateway_credentials() else {
                return http::Response::builder().status(401).body(b"auth required".to_vec()).unwrap_or_default();
            };
            let url = format!("{}/api/client/v1/apps/{app_id}/assets/{asset}", base_url.trim_end_matches('/'));
            let client = reqwest::blocking::Client::new();
            let Ok(response) = native_client::apply_headers(client.get(url).bearer_auth(token)).send() else {
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
            // The long-lived event bridge must be kernel-owned: register it through
            // spawn_task from a one-shot Tauri-runtime hop (spawn_task needs a runtime
            // context). Kernel shutdown aborts/disposes it like every other task.
            let event_bootstrap = Arc::clone(&setup_bootstrap_events);
            tauri::async_runtime::spawn(async move {
                let task = event_bootstrap.spawn_task("zk.desktop.tauri-events", async move {
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
                if let Err(error) = task {
                    eprintln!("zk-desktop: event bridge not kernel-owned: {error}");
                }
            });
            if let Ok(app_id) = std::env::var("ZEOKEEL_OPEN_APP_ID") {
                let handle = app.handle().clone();
                let open_bootstrap = Arc::clone(&setup_bootstrap_open);
                tauri::async_runtime::spawn(async move {
                    let task = open_bootstrap.spawn_task("zk.desktop.open-app", async move {
                        if let Err(error) = open_app_window_impl(&handle, &app_id, "index.html") {
                            eprintln!("zk-desktop: open app window failed: {error}");
                        }
                    });
                    if let Err(error) = task {
                        eprintln!("zk-desktop: open-app task not registered: {error}");
                    }
                });
            }
            Ok(())
        });
    // `cargo test` builds this bin with `cfg(test)` in a checkout without frontend
    // assets; `generate_context!` (which embeds `web/dist` and icons) is only required
    // for the real binary, which CI/release build after `pnpm --dir apps/desktop/web build`.
    #[cfg(not(test))]
    let result = builder
        .run(tauri::generate_context!())
        .map_err(|error| format!("failed to run window: {error}"));
    #[cfg(test)]
    let result: Result<(), String> = {
        let _ = builder;
        Ok(())
    };
    // Desktop exit: dispose kernel effects (reverse order, exactly once).
    if let Err(error) = bootstrap.shutdown() {
        eprintln!("zk-desktop: bootstrap shutdown failed: {error}");
    }
    result
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only
    #[test]
    fn app_capability_has_no_core_permissions() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities/app-window.json");
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(value["windows"], serde_json::json!(["zk-app-*"]));
        assert_eq!(value["permissions"], serde_json::json!([]));
    }

    #[test]
    fn app_id_validation() {
        assert!(super::valid_app_id("app-1"));
        assert!(!super::valid_app_id(".."));
        assert!(!super::valid_app_id("a/b"));
    }
}
