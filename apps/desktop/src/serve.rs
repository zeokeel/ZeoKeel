#![forbid(unsafe_code)]
//! HTTP serve carrier for e2e/remote use (same dispatch as the Tauri shell).
//!
//! Security: bound to loopback, a per-process random cookie token protects
//! `/ipc` and `/events`, CORS only allows the local UI origins, and the index
//! response sets the HttpOnly/SameSite cookie the browser then sends back.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio::sync::broadcast;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use zk_contract_ipc::{CommandRequest, CommandResponse};

use crate::ipc::{dispatch, AppState};

const COOKIE_NAME: &str = "zk_serve_token";

/// State shared by the HTTP routes (dispatch state + per-process auth token + dist dir).
#[derive(Clone)]
struct ServeState {
    app: Arc<AppState>,
    token: String,
    dist: String,
}

/// Run the HTTP carrier until the process exits.
pub fn run(state: Arc<AppState>, addr: &str) -> Result<(), String> {
    let listener = std::net::TcpListener::bind(addr).map_err(|error| format!("bind {addr}: {error}"))?;
    listener.set_nonblocking(true).map_err(|error| error.to_string())?;
    let dist = std::env::var("ZEOKEEL_WEB_DIST").unwrap_or_else(|_| "apps/desktop/web/dist".to_owned());
    let state = ServeState { app: state, token: uuid::Uuid::new_v4().to_string(), dist };
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

fn router(state: ServeState) -> Router {
    let index_path = format!("{}/index.html", state.dist);
    let static_files = ServeDir::new(&state.dist).not_found_service(ServeFile::new(index_path));
    let allowed = [
        HeaderValue::from_static("http://127.0.0.1:8788"),
        HeaderValue::from_static("http://localhost:8788"),
    ];
    Router::new()
        .route("/healthz", get(healthz))
        .route("/", get(index))
        .route("/index.html", get(index))
        .route("/ipc", post(ipc))
        .route("/events", get(events))
        .fallback_service(static_files)
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(allowed))
                .allow_credentials(true)
                .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]),
        )
        .with_state(state)
}

fn authorized(headers: &HeaderMap, token: &str) -> bool {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|value| value == token);
    if bearer {
        return true;
    }
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|cookies| {
            cookies.split(';').any(|cookie| {
                cookie.trim().split_once('=').is_some_and(|(name, value)| name == COOKIE_NAME && value == token)
            })
        })
}

async fn healthz(State(state): State<ServeState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "edition": zk_composed::EDITION,
        "packages": zk_composed::PACKAGES,
        "runtime_ready": !state.app.runtime().packages().is_empty(),
    }))
}

async fn index(State(state): State<ServeState>) -> Response {
    let path = std::path::Path::new(&state.dist).join("index.html");
    let Ok(body) = std::fs::read_to_string(path) else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "web dist missing").into_response();
    };
    let cookie = format!("{COOKIE_NAME}={}; HttpOnly; SameSite=Strict; Path=/", state.token);
    let mut response = Html(body).into_response();
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}

async fn ipc(
    State(state): State<ServeState>,
    headers: HeaderMap,
    Json(request): Json<CommandRequest>,
) -> Response {
    if !authorized(&headers, &state.token) {
        return (StatusCode::UNAUTHORIZED, Json(CommandResponse::err("unauthorized"))).into_response();
    }
    Json(dispatch(&state.app, &request.name, &request.payload)).into_response()
}

async fn events(
    State(state): State<ServeState>,
    headers: HeaderMap,
) -> Response {
    if !authorized(&headers, &state.token) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    let receiver = state.app.bus.subscribe();
    let stream = futures::stream::unfold(receiver, |mut receiver| async move {
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_owned());
                    return Some((Ok::<_, std::convert::Infallible>(Event::default().data(json)), receiver));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default()).into_response()
}
