//! Enterprise update manifest/download (`srv.updates`).
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use zk_kit_server::{error, AppState, ServerError};

/// Update routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/updates/manifest", get(manifest))
        .route("/api/updates/client/{artifact}", get(download))
        .route("/portal", get(portal))
}

fn update_dir() -> PathBuf { PathBuf::from(std::env::var("ZEOKEEL_UPDATE_DIR").unwrap_or_else(|_| "deploy/updates".to_owned())) }

fn release() -> Result<Value, ServerError> {
    let path = update_dir().join("CLIENT-RELEASE.json");
    let text = std::fs::read_to_string(path).map_err(|_| error(StatusCode::NOT_FOUND, "UPDATE_UNAVAILABLE"))?;
    serde_json::from_str(&text).map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "UPDATE_UNAVAILABLE"))
}

fn client_header(headers: &HeaderMap) -> Option<(String, String, String)> {
    let raw = headers.get("x-zeokeel-client")?.to_str().ok()?;
    let mut parts = raw.split(';');
    let version = parts.next()?.trim().to_owned();
    let channel = parts.next()?.trim().to_owned();
    let platform = parts.next()?.trim().to_owned();
    if version.is_empty() || channel.is_empty() || platform.is_empty() { None } else { Some((version, channel, platform)) }
}

async fn portal() -> Response {
    let release = release().unwrap_or_else(|_| json!({ "channel_id": "official", "client_version": "0.1.0" }));
    let channel = release.get("channel_id").and_then(Value::as_str).unwrap_or("official");
    let version = release.get("client_version").and_then(Value::as_str).unwrap_or("0.1.0");
    let channel_json: Value = std::fs::read_to_string(update_dir().join("CHANNEL.json")).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_else(|| json!({}));
    let product = channel_json.get("product_name").and_then(Value::as_str).unwrap_or(channel);
    let powered_by = channel_json.get("powered_by").and_then(Value::as_str).unwrap_or("hide");
    let footer = if powered_by == "show" { "<p><small>Powered by ZeoKeel</small></p>" } else { "" };
    let html = format!("<!doctype html><html lang=zh-CN><head><meta charset=utf-8><title>{product} 下载</title><style>body{{font-family:system-ui;margin:40px;max-width:720px}}a{{display:inline-block;margin:8px 0}}</style></head><body><h1>{product} 客户端下载</h1><p>渠道：{channel} · 版本：{version}</p><p><a href=\"/api/updates/client/{product}_{version}_linux_x64.AppImage\">Linux AppImage</a></p><p><small>Windows 安装包未签名，请核对 SHA256。企业版客户端只从此服务端更新。</small></p>{footer}</body></html>");
    let mut response = html.into_response();
    response.headers_mut().insert(axum::http::header::CONTENT_TYPE, axum::http::HeaderValue::from_static("text/html; charset=utf-8"));
    response.headers_mut().insert(axum::http::header::CONTENT_SECURITY_POLICY, axum::http::HeaderValue::from_static("default-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'"));
    response
}

async fn manifest(State(_state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>, ServerError> {
    let Some((client_version, channel, platform)) = client_header(&headers) else {
        return Err(error(StatusCode::BAD_REQUEST, "CLIENT_HEADER_REQUIRED"));
    };
    let mut release = release()?;
    let release_channel = release.get("channel_id").and_then(Value::as_str).unwrap_or_default();
    let release_edition = release.get("edition").and_then(Value::as_str).unwrap_or("enterprise");
    if channel != release_channel { return Err(error(StatusCode::CONFLICT, "CHANNEL_MISMATCH")); }
    if release_edition != "enterprise" { return Err(error(StatusCode::CONFLICT, "EDITION_MISMATCH")); }
    let min_supported = release.get("min_supported").and_then(Value::as_str).unwrap_or("").to_owned();
    let current_client = release.get("client_version").and_then(Value::as_str).unwrap_or("").to_owned();
    if client_version != current_client { return Err(error(StatusCode::CONFLICT, "CLIENT_VERSION_MISMATCH")); }
    let forced = false;
    let previous = release.get("previous").cloned().unwrap_or(Value::Null);
    let object = release.as_object_mut().ok_or_else(|| error(StatusCode::INTERNAL_SERVER_ERROR, "UPDATE_UNAVAILABLE"))?;
    object.insert("client".into(), json!({ "version": client_version, "channel": channel, "platform": platform }));
    object.insert("forced".into(), json!(forced));
    object.insert("min_supported".into(), json!(min_supported));
    object.insert("previous".into(), previous);
    Ok(Json(release))
}

async fn download(State(_state): State<AppState>, AxumPath(artifact): AxumPath<String>) -> Result<Response, ServerError> {
    if artifact.is_empty() || artifact.contains("..") || artifact.contains('/') || artifact.contains('\\') {
        return Err(error(StatusCode::BAD_REQUEST, "VALIDATION"));
    }
    let path = update_dir().join("clients").join(&artifact);
    let bytes = std::fs::read(&path).map_err(|_| error(StatusCode::NOT_FOUND, "NOT_FOUND"))?;
    let mut response = bytes.into_response();
    response.headers_mut().insert(axum::http::header::CONTENT_TYPE, axum::http::HeaderValue::from_static("application/octet-stream"));
    Ok(response)
}

/// Read a release from an arbitrary dir (tests/tooling).
pub fn read_release(dir: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(dir.join("CLIENT-RELEASE.json")).map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}
