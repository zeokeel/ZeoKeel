#![forbid(unsafe_code)]
//! HTTP serve carrier for e2e/remote use (same dispatch as the Tauri shell).

use std::sync::Arc;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::stream::Stream;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use zk_contract_ipc::{CommandRequest, CommandResponse};

use crate::ipc::{dispatch, AppState};

/// Run the HTTP carrier until the process exits.
pub fn run(state: Arc<AppState>, addr: &str) -> Result<(), String> {
    let listener = std::net::TcpListener::bind(addr).map_err(|error| format!("bind {addr}: {error}"))?;
    listener.set_nonblocking(true).map_err(|error| error.to_string())?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    runtime.block_on(async move {
        let app = router(state);
        axum::serve(tokio::net::TcpListener::from_std(listener).map_err(|e| e.to_string())?, app)
            .await
            .map_err(|error| error.to_string())
    })
}

fn router(state: Arc<AppState>) -> Router {
    let dist = std::env::var("ZEOKEEL_WEB_DIST").unwrap_or_else(|_| "apps/desktop/web/dist".to_owned());
    let index = format!("{dist}/index.html");
    let static_files = ServeDir::new(&dist).not_found_service(ServeFile::new(index));
    Router::new()
        .route("/healthz", get(healthz))
        .route("/ipc", post(ipc))
        .route("/events", get(events))
        .fallback_service(static_files)
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn healthz(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "edition": zk_composed::EDITION,
        "packages": zk_composed::PACKAGES,
        "runtime_ready": !state.runtime().packages().is_empty(),
    }))
}

async fn ipc(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CommandRequest>,
) -> Json<CommandResponse> {
    Json(dispatch(&state, &request.name, &request.payload))
}

async fn events(State(state): State<Arc<AppState>>) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let receiver = state.bus.subscribe();
    let stream = futures::stream::unfold(receiver, |mut receiver| async move {
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_owned());
                    return Some((Ok(Event::default().data(json)), receiver));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
