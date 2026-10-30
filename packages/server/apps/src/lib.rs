//! WASM app publishing/lifecycle (`srv.apps`).
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Mutex, OnceLock};

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;
use tokio::sync::Semaphore;
use uuid::Uuid;
use zk_contract_app::{AppManifest, AppOpen, AppRequest, ABI};
use zk_kit_server::{audit, current_user, error, require_admin, AppState, ServerError};
use zk_srv_app_db as app_db;
use zk_srv_app_runtime as runtime;

/// Migrations owned by this package.
pub const MIGRATIONS: &[(&str, &str)] = &[
    ("0012_apps", include_str!("../migrations/0012_apps.sql")),
    ("0013_app_logs", include_str!("../migrations/0013_app_logs.sql")),
];

const MAX_WASM: usize = 32 * 1024 * 1024;
const MAX_UI: usize = 16 * 1024 * 1024;

fn app_dbs() -> &'static Mutex<HashMap<Uuid, Arc<Semaphore>>> {
    static VALUES: OnceLock<Mutex<HashMap<Uuid, Arc<Semaphore>>>> = OnceLock::new();
    VALUES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn user_sems() -> &'static Mutex<HashMap<Uuid, Arc<Semaphore>>> {
    static VALUES: OnceLock<Mutex<HashMap<Uuid, Arc<Semaphore>>>> = OnceLock::new();
    VALUES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn semaphore(app: Uuid, capacity: usize) -> Arc<Semaphore> {
    let Ok(mut map) = app_dbs().lock() else { return Arc::new(Semaphore::new(1)); };
    map.entry(app).or_insert_with(|| Arc::new(Semaphore::new(capacity.max(1)))).clone()
}

fn user_semaphore(user: Uuid) -> Arc<Semaphore> {
    let Ok(mut map) = user_sems().lock() else { return Arc::new(Semaphore::new(1)); };
    map.entry(user).or_insert_with(|| Arc::new(Semaphore::new(8))).clone()
}

fn global_semaphore() -> &'static Arc<Semaphore> {
    static VALUES: OnceLock<Arc<Semaphore>> = OnceLock::new();
    VALUES.get_or_init(|| Arc::new(Semaphore::new(32)))
}

/// App routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/client/v1/apps", get(catalog).post(upload))
        .route("/api/client/v1/apps/{id}/open", post(open))
        .route("/api/client/v1/apps/{id}/request", post(request).layer(DefaultBodyLimit::max(1024 * 1024)))
        .route("/api/client/v1/apps/{id}/assets/{*path}", get(asset))
        .route("/api/admin/apps", get(admin_list))
        .route("/api/admin/apps/{id}/{action}", post(admin_action))
        .route("/api/admin/apps/{id}/grants", put(admin_grants))
        .route("/api/admin/apps/{id}/transfer", post(admin_transfer))
        .layer(DefaultBodyLimit::max(MAX_WASM + MAX_UI + 1024 * 1024))
}

fn sha256(bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(bytes);
    format!("{:x}", hash.finalize())
}

fn uuid_of(value: &str) -> Result<Uuid, ServerError> { Uuid::parse_str(value).map_err(|_| error(StatusCode::BAD_REQUEST, "VALIDATION")) }

async fn upload(State(state): State<AppState>, headers: HeaderMap, mut multipart: Multipart) -> Result<Json<Value>, ServerError> {
    let (user_id, role, _epoch) = current_user(&state, &headers).await?;
    if role == "auditor" { return Err(error(StatusCode::FORBIDDEN, "APP_FORBIDDEN")); }
    let owner = uuid_of(&user_id)?;
    let mut manifest_raw: Option<String> = None;
    let mut wasm: Option<Bytes> = None;
    let mut ui: Option<Bytes> = None;
    while let Some(field) = multipart.next_field().await.map_err(|_| error(StatusCode::BAD_REQUEST, "VALIDATION"))? {
        match field.name() {
            Some("manifest") => manifest_raw = Some(field.text().await.map_err(|_| error(StatusCode::BAD_REQUEST, "VALIDATION"))?),
            Some("wasm") => wasm = Some(field.bytes().await.map_err(|_| error(StatusCode::PAYLOAD_TOO_LARGE, "VALIDATION"))?),
            Some("ui") => ui = Some(field.bytes().await.map_err(|_| error(StatusCode::PAYLOAD_TOO_LARGE, "VALIDATION"))?),
            _ => {}
        }
    }
    let manifest: AppManifest = serde_json::from_str(&manifest_raw.ok_or_else(|| error(StatusCode::BAD_REQUEST, "APP_BAD_MANIFEST"))?).map_err(|_| error(StatusCode::BAD_REQUEST, "APP_BAD_MANIFEST"))?;
    let wasm = wasm.ok_or_else(|| error(StatusCode::BAD_REQUEST, "APP_BAD_WASM"))?;
    if wasm.len() > MAX_WASM || wasm.get(..4) != Some(b"\0asm") || manifest.version.trim().is_empty() || manifest.name.trim().is_empty() {
        return Err(error(StatusCode::BAD_REQUEST, "APP_BAD_WASM"));
    }
    if let Some(ui) = &ui {
        if ui.len() > MAX_UI { return Err(error(StatusCode::PAYLOAD_TOO_LARGE, "APP_BAD_MANIFEST")); }
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(ui.as_ref())).map_err(|_| error(StatusCode::BAD_REQUEST, "APP_BAD_MANIFEST"))?;
        let mut names = Vec::new();
        for index in 0..archive.len() {
            let entry = archive.by_index(index).map_err(|_| error(StatusCode::BAD_REQUEST, "APP_BAD_MANIFEST"))?;
            let name = entry.name().trim_start_matches('/').to_owned();
            if !zk_kit_content::is_safe_archive_path(&name) { return Err(error(StatusCode::BAD_REQUEST, "APP_BAD_MANIFEST")); }
            names.push(name);
        }
        if manifest.ui.iter().any(|name| !names.contains(name)) { return Err(error(StatusCode::BAD_REQUEST, "APP_BAD_MANIFEST")); }
    }
    let existing: Option<(Uuid, Uuid)> = sqlx::query_as("SELECT id, owner_id FROM contents WHERE kind = 'app' AND name = $1 AND deleted_at IS NULL")
        .bind(manifest.name.trim()).fetch_optional(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let content = match existing {
        Some((id, owner_id)) if owner_id == owner => id,
        Some(_) => return Err(error(StatusCode::FORBIDDEN, "APP_FORBIDDEN")),
        None => sqlx::query_scalar("INSERT INTO contents (kind, name, owner_id, status) VALUES ('app', $1, $2, 'pending') RETURNING id")
            .bind(manifest.name.trim()).bind(owner).fetch_one(&state.pool).await.map_err(|_| error(StatusCode::CONFLICT, "CONFLICT"))?,
    };
    let wasm_sha = sha256(&wasm);
    let ui_sha = ui.as_ref().map(|bytes| sha256(bytes)).unwrap_or_default();
    let release: Uuid = sqlx::query_scalar(
        "INSERT INTO content_releases (content_id, version, sha256, archive_size, archive, changelog, created_by, wasm, wasm_sha256, ui_archive, ui_sha256, ui_manifest, db_schema_version, db_migrations)
         VALUES ($1, $2, $3, $4, $5, '', $6, $5, $3, $7, $8, $9, $10, $11) RETURNING id",
    ).bind(content).bind(manifest.version.trim()).bind(&wasm_sha).bind(i64::try_from(wasm.len()).unwrap_or(0)).bind(wasm.as_ref()).bind(owner)
        .bind(ui.as_ref().map(|bytes| bytes.as_ref())).bind(&ui_sha).bind(json!(manifest.ui)).bind(i64::from(manifest.db_schema_version)).bind(json!(manifest.db_migrations))
        .fetch_one(&state.pool).await.map_err(|_| error(StatusCode::CONFLICT, "CONFLICT"))?;
    sqlx::query("UPDATE contents SET current_release_id = $1, status = 'pending', updated_at = now() WHERE id = $2").bind(release).bind(content).execute(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    audit(&state, owner, "app.upload", &content.to_string(), &json!({ "name": manifest.name, "version": manifest.version, "wasm_sha256": wasm_sha })).await?;
    Ok(Json(json!({ "ok": true, "app_id": content.to_string(), "status": "pending" })))
}

async fn catalog(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>, ServerError> {
    let (user_id, _role, _epoch) = current_user(&state, &headers).await?;
    let user = uuid_of(&user_id)?;
    let rows = sqlx::query(
        "SELECT c.id, c.name, c.visibility, r.version, c.force_update, r.ui_manifest
         FROM contents c JOIN content_releases r ON r.id = c.current_release_id
         WHERE c.kind = 'app' AND c.status = 'approved' AND c.deleted_at IS NULL
           AND (c.visibility = 'all' OR EXISTS (SELECT 1 FROM content_grants g WHERE g.content_id = c.id AND g.grantee = $1 AND g.can_install))
         ORDER BY c.name",
    ).bind(user.to_string()).fetch_all(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let items: Vec<Value> = rows.into_iter().map(|row| json!({
        "app_id": row.try_get::<Uuid, _>("id").map(|value| value.to_string()).unwrap_or_default(),
        "name": row.try_get::<String, _>("name").unwrap_or_default(),
        "description": "", "icon": "",
        "version": row.try_get::<String, _>("version").unwrap_or_default(),
        "access": row.try_get::<String, _>("visibility").unwrap_or_default(),
        "ui": row.try_get::<Value, _>("ui_manifest").is_ok(),
    })).collect();
    Ok(Json(json!({ "ok": true, "items": items })))
}

async fn open(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> Result<Response, ServerError> {
    let (user_id, _role, _epoch) = current_user(&state, &headers).await?;
    let user = uuid_of(&user_id)?;
    let app = uuid_of(&id)?;
    let row = sqlx::query(
        "SELECT c.id, c.status, c.deleted_at, r.version, r.ui_manifest FROM contents c LEFT JOIN content_releases r ON r.id = c.current_release_id
         WHERE c.id = $1 AND c.kind = 'app' AND EXISTS (SELECT 1 FROM content_grants g WHERE g.content_id = c.id AND g.grantee IN ('all', $2) AND g.can_install)",
    ).bind(app).bind(user.to_string()).fetch_optional(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "APP_NOT_FOUND"))?;
    let status: String = row.try_get("status").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let deleted: Option<chrono::DateTime<chrono::Utc>> = row.try_get("deleted_at").ok();
    if status != "approved" || deleted.is_some() { return Err(error(StatusCode::FORBIDDEN, "APP_DELISTED")); }
    let version: String = row.try_get("version").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let manifest: Value = row.try_get("ui_manifest").unwrap_or(json!([]));
    audit(&state, user, "app.open", &app.to_string(), &json!({})).await?;
    let payload = AppOpen { app_id: app.to_string(), version: version.clone(), ui_manifest: manifest.to_string() };
    let mut response = Json(payload).into_response();
    response.headers_mut().insert("x-zeokeel-app-version", HeaderValue::from_str(&version).map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?);
    response.headers_mut().insert("x-zeokeel-app-ui", HeaderValue::from_static("static"));
    Ok(response)
}


async fn request(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>, Json(body): Json<AppRequest>) -> Result<Json<Value>, ServerError> {
    if body.app_id != id { return Err(error(StatusCode::BAD_REQUEST, "APP_BAD_REQUEST")); }
    let (user_id, _role, _epoch) = current_user(&state, &headers).await?;
    let user = uuid_of(&user_id)?;
    let app = uuid_of(&id)?;
    let row = sqlx::query(
        "SELECT c.status, c.deleted_at, r.wasm, r.ui_archive, r.db_schema_version, r.db_migrations FROM contents c JOIN content_releases r ON r.id = c.current_release_id
         WHERE c.id = $1 AND c.kind = 'app' AND EXISTS (SELECT 1 FROM content_grants g WHERE g.content_id = c.id AND g.grantee IN ('all', $2) AND g.can_install)",
    ).bind(app).bind(user.to_string()).fetch_optional(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "APP_NOT_FOUND"))?;
    if row.try_get::<String, _>("status").unwrap_or_default() != "approved" || row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("deleted_at").ok().flatten().is_some() { return Err(error(StatusCode::GONE, "APP_DELISTED")); }
    if !body.abi.is_empty() && body.abi != ABI { return Err(error(StatusCode::BAD_REQUEST, "APP_BAD_REQUEST")); }
    let wasm: Vec<u8> = row.try_get("wasm").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let schema_version: i32 = row.try_get("db_schema_version").unwrap_or(0);
    let migrations: Vec<zk_contract_app::AppMigration> = serde_json::from_value(row.try_get("db_migrations").unwrap_or(json!([]))).unwrap_or_default();
    let ui_archive: Option<Vec<u8>> = row.try_get("ui_archive").ok();
    let mut assets: HashMap<String, Vec<u8>> = HashMap::new();
    if let Some(archive) = ui_archive {
        if let Ok(mut zip) = zip::ZipArchive::new(std::io::Cursor::new(archive)) {
            for index in 0..zip.len() {
                if let Ok(mut entry) = zip.by_index(index) {
                    let name = entry.name().to_owned();
                    let mut bytes = Vec::new();
                    if entry.read_to_end(&mut bytes).is_ok() { assets.insert(name, bytes); }
                }
            }
        }
    }
    let db_root = std::env::var("ZEOKEEL_APP_DB_DIR").unwrap_or_else(|_| "target/app-db".to_owned());
    std::fs::create_dir_all(&db_root).map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let db_path = app_db::db_path(std::path::Path::new(&db_root), &app.to_string());
    app_db::ensure_schema(&db_path, u32::try_from(schema_version).unwrap_or(0), &migrations).map_err(|code| error(StatusCode::INTERNAL_SERVER_ERROR, match code.as_str() { "APP_DB_TABLES" => "APP_DB_TABLES", _ => "APP_BAD_REQUEST" }))?;
    app_db::enforce_quota(&db_path, app_db::MAX_BYTES).map_err(|_| error(StatusCode::TOO_MANY_REQUESTS, "APP_DB_QUOTA"))?;
    let decoded = base64::engine::general_purpose::STANDARD.decode(&body.body_b64).map_err(|_| error(StatusCode::BAD_REQUEST, "APP_BAD_REQUEST"))?;
    if decoded.len() > 1024 * 1024 { return Err(error(StatusCode::PAYLOAD_TOO_LARGE, "APP_BAD_REQUEST")); }
    let guest_body: Value = serde_json::from_slice(&decoded).unwrap_or(Value::Null);
    let request = json!({ "abi": ABI, "app_id": app.to_string(), "method": body.method, "path": body.path, "query": body.query, "body": guest_body, "request_id": body.request_id });
    let limits: Option<(i64, i64, i64)> = sqlx::query_as("SELECT fuel, max_concurrency, max_memory_bytes FROM app_runtime_limits WHERE app_id=$1").bind(app).fetch_optional(&state.pool).await.ok().flatten();
    let (fuel, max_concurrency, max_memory) = limits.unwrap_or((10_000_000, 4, 64 * 1024 * 1024));
    let app_permit = tokio::time::timeout(std::time::Duration::from_secs(5), semaphore(app, usize::try_from(max_concurrency).unwrap_or(4)).acquire_owned()).await.map_err(|_| error(StatusCode::TOO_MANY_REQUESTS, "APP_CONCURRENCY"))?.map_err(|_| error(StatusCode::TOO_MANY_REQUESTS, "APP_CONCURRENCY"))?;
    let user_permit = tokio::time::timeout(std::time::Duration::from_secs(5), user_semaphore(user).acquire_owned()).await.map_err(|_| error(StatusCode::TOO_MANY_REQUESTS, "APP_CONCURRENCY"))?.map_err(|_| error(StatusCode::TOO_MANY_REQUESTS, "APP_CONCURRENCY"))?;
    let global_permit = tokio::time::timeout(std::time::Duration::from_secs(5), global_semaphore().clone().acquire_owned()).await.map_err(|_| error(StatusCode::TOO_MANY_REQUESTS, "APP_CONCURRENCY"))?.map_err(|_| error(StatusCode::TOO_MANY_REQUESTS, "APP_CONCURRENCY"))?;
    let _ = sqlx::query("INSERT INTO app_databases (app_id, schema_version, size_bytes) VALUES ($1,$2,$3) ON CONFLICT (app_id) DO UPDATE SET schema_version=EXCLUDED.schema_version, size_bytes=EXCLUDED.size_bytes, updated_at=now()")
        .bind(app).bind(schema_version).bind(i64::try_from(std::fs::metadata(&db_path).map(|meta| meta.len()).unwrap_or(0)).unwrap_or(0)).execute(&state.pool).await;
    let db_for_run = db_path.clone();
    let result = tokio::time::timeout(std::time::Duration::from_secs(60), tokio::task::spawn_blocking(move || {
        let _app = app_permit;
        let _user = user_permit;
        let _global = global_permit;
        runtime::execute(&wasm, &db_for_run, request, u64::try_from(fuel).unwrap_or(10_000_000), usize::try_from(max_memory).unwrap_or(64 * 1024 * 1024), &assets)
    })).await.map_err(|_| error(StatusCode::TOO_MANY_REQUESTS, "APP_LIMIT_TIME"))?.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "APP_UNAVAILABLE"))?;
    let output = result.map_err(|failure| {
        let code = runtime::error_code(&failure);
        let status = if code.starts_with("APP_LIMIT") { StatusCode::TOO_MANY_REQUESTS } else { StatusCode::BAD_REQUEST };
        error(status, code)
    })?;
    sqlx::query("INSERT INTO app_calls (app_id, user_id, request_id, status, duration_ms, fuel) VALUES ($1,$2,$3,$4,$5,$6)")
        .bind(app).bind(user).bind(&body.request_id).bind(i64::try_from(output.response.get("status").and_then(Value::as_u64).unwrap_or(500)).unwrap_or(500))
        .bind(i64::try_from(output.elapsed.as_millis()).unwrap_or(0)).bind(i64::try_from(output.fuel_used).unwrap_or(0)).execute(&state.pool).await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "AUDIT_FAILED"))?;
    audit(&state, user, "app.request", &format!("{}:{}", app, body.request_id), &json!({ "status": output.response.get("status") })).await?;
    let _ = sqlx::query("UPDATE app_databases SET size_bytes=$1, updated_at=now() WHERE app_id=$2").bind(i64::try_from(std::fs::metadata(&db_path).map(|meta| meta.len()).unwrap_or(0)).unwrap_or(0)).bind(app).execute(&state.pool).await;
    if app_db::enforce_quota(&db_path, app_db::MAX_BYTES).is_err() { return Err(error(StatusCode::TOO_MANY_REQUESTS, "APP_DB_QUOTA")); }
    let total: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(size_bytes),0)::bigint FROM app_databases").fetch_one(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if total > 20 * 1024 * 1024 * 1024 { return Err(error(StatusCode::TOO_MANY_REQUESTS, "APP_DB_QUOTA")); }
    let response_body = output.response.get("body").cloned().unwrap_or(Value::Null);
    let encoded = base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(&response_body).unwrap_or_default());
    if encoded.len() > 8 * 1024 * 1024 { return Err(error(StatusCode::PAYLOAD_TOO_LARGE, "APP_BAD_REQUEST")); }
    Ok(Json(json!({ "status": output.response.get("status").and_then(Value::as_u64).unwrap_or(200), "headers": {}, "body_b64": encoded })))
}

async fn asset(State(state): State<AppState>, headers: HeaderMap, Path((id, path)): Path<(String, String)>) -> Result<Response, ServerError> {
    let (user_id, _role, _epoch) = current_user(&state, &headers).await?;
    let user = uuid_of(&user_id)?;
    let app = uuid_of(&id)?;
    if path.contains("..") || path.contains(':') { return Err(error(StatusCode::BAD_REQUEST, "APP_BAD_REQUEST")); }
    let row = sqlx::query("SELECT r.ui_archive, r.ui_manifest FROM contents c JOIN content_releases r ON r.id=c.current_release_id WHERE c.id=$1 AND c.status='approved' AND c.deleted_at IS NULL AND EXISTS (SELECT 1 FROM content_grants g WHERE g.content_id=c.id AND g.grantee IN ('all',$2) AND g.can_install)")
        .bind(app).bind(user.to_string()).fetch_optional(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let Some(row) = row else { return Err(error(StatusCode::NOT_FOUND, "APP_NOT_FOUND")); };
    let archive: Option<Vec<u8>> = row.try_get("ui_archive").ok();
    let manifest: Value = row.try_get("ui_manifest").unwrap_or(json!([]));
    let allowed = manifest.as_array().map(|items| items.iter().any(|item| item.as_str() == Some(&path))).unwrap_or(false);
    if !allowed { return Err(error(StatusCode::NOT_FOUND, "APP_NOT_FOUND")); }
    let Some(archive) = archive else { return Err(error(StatusCode::NOT_FOUND, "APP_NOT_FOUND")); };
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive)).map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let Ok(mut entry) = zip.by_name(&path) else { return Err(error(StatusCode::NOT_FOUND, "APP_NOT_FOUND")); };
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let content_type = if path.ends_with(".js") { "text/javascript; charset=utf-8" } else if path.ends_with(".css") { "text/css; charset=utf-8" } else if path.ends_with(".svg") { "image/svg+xml" } else if path.ends_with(".json") { "application/json" } else { "text/html; charset=utf-8" };
    let mut response = bytes.into_response();
    response.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_str(content_type).map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?);
    Ok(response)
}

async fn admin_list(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>, ServerError> {
    require_admin(&state, &headers, false).await?;
    let rows = sqlx::query("SELECT c.id, c.name, c.status, c.owner_id, r.version, r.wasm_sha256 FROM contents c LEFT JOIN content_releases r ON r.id=c.current_release_id WHERE c.kind='app' AND c.deleted_at IS NULL ORDER BY c.created_at DESC")
        .fetch_all(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let items: Vec<Value> = rows.into_iter().map(|row| json!({
        "id": row.try_get::<Uuid,_>("id").map(|v| v.to_string()).unwrap_or_default(),
        "name": row.try_get::<String,_>("name").unwrap_or_default(),
        "status": row.try_get::<String,_>("status").unwrap_or_default(),
        "owner_id": row.try_get::<Uuid,_>("owner_id").map(|v| v.to_string()).unwrap_or_default(),
        "version": row.try_get::<Option<String>,_>("version").ok().flatten().unwrap_or_default(),
        "wasm_sha256": row.try_get::<Option<String>,_>("wasm_sha256").ok().flatten().unwrap_or_default(),
    })).collect();
    Ok(Json(json!({ "ok": true, "items": items })))
}

async fn admin_action(State(state): State<AppState>, headers: HeaderMap, Path((id, action)): Path<(String, String)>) -> Result<Json<Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    let app = uuid_of(&id)?;
    if !matches!(action.as_str(), "approve" | "reject" | "delist" | "rollback" | "delete" | "purge") {
        return Err(error(StatusCode::BAD_REQUEST, "VALIDATION"));
    }
    if action == "purge" {
        let role: String = sqlx::query_scalar("SELECT role FROM users WHERE id=$1").bind(admin.user_id).fetch_one(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
        if role != "super_admin" { return Err(error(StatusCode::FORBIDDEN, "FORBIDDEN")); }
        let file = std::path::Path::new(&std::env::var("ZEOKEEL_APP_DB_DIR").unwrap_or_else(|_| "target/app-db".to_owned())).join(format!("{app}.sqlite"));
        let _ = std::fs::remove_file(file);
        sqlx::query("DELETE FROM contents WHERE id=$1 AND kind='app'").bind(app).execute(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
        audit(&state, admin.user_id, "app.purge", &app.to_string(), &json!({})).await?;
        return Ok(Json(json!({ "ok": true })));
    }
    if action == "rollback" {
        let target: Option<(Uuid, i32)> = sqlx::query_as("SELECT id, db_schema_version FROM content_releases WHERE content_id=$1 ORDER BY created_at DESC OFFSET 1 LIMIT 1").bind(app).fetch_optional(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
        let Some((target, target_schema)) = target else { return Err(error(StatusCode::CONFLICT, "APP_NOT_FOUND")); };
        let current_schema: i32 = sqlx::query_scalar("SELECT db_schema_version FROM content_releases WHERE content_id=$1 ORDER BY created_at DESC LIMIT 1").bind(app).fetch_one(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
        if target_schema != current_schema { return Err(error(StatusCode::CONFLICT, "APP_DB_SCHEMA")); }
        sqlx::query("UPDATE contents SET current_release_id=$1, status='approved', updated_at=now() WHERE id=$2").bind(target).bind(app).execute(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
        sqlx::query("INSERT INTO content_deployments (content_id, release_id, action, actor_id, reason) VALUES ($1,$2,'rollback',$3,'')").bind(app).bind(target).bind(admin.user_id).execute(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
        audit(&state, admin.user_id, "app.rollback", &app.to_string(), &json!({ "release_id": target.to_string() })).await?;
        return Ok(Json(json!({ "ok": true })));
    }
    if action == "delete" {
        sqlx::query("UPDATE contents SET status='delisted', deleted_at=now(), updated_at=now() WHERE id=$1").bind(app).execute(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
        sqlx::query("INSERT INTO content_deployments (content_id, release_id, action, actor_id, reason) VALUES ($1,NULL,'delete',$2,'')").bind(app).bind(admin.user_id).execute(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
        audit(&state, admin.user_id, "app.delete", &app.to_string(), &json!({})).await?;
        return Ok(Json(json!({ "ok": true })));
    }
    let status = match action.as_str() { "approve" => "approved", "reject" => "rejected", _ => "delisted" };
    let result = sqlx::query("UPDATE contents SET status=$1, updated_at=now() WHERE id=$2 AND kind='app' AND deleted_at IS NULL").bind(status).bind(app).execute(&state.pool).await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if result.rows_affected() == 0 { return Err(error(StatusCode::NOT_FOUND, "APP_NOT_FOUND")); }
    if action == "approve" { let _ = sqlx::query("INSERT INTO content_deployments (content_id, release_id, action, actor_id, reason) SELECT $1, current_release_id, 'publish', $2, '' FROM contents WHERE id=$1").bind(app).bind(admin.user_id).execute(&state.pool).await; }
    audit(&state, admin.user_id, &format!("app.{action}"), &app.to_string(), &json!({})).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
struct Transfer { user_id: String }

async fn admin_transfer(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>, Json(body): Json<Transfer>) -> Result<Json<Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    let app = uuid_of(&id)?;
    let owner = uuid_of(&body.user_id)?;
    sqlx::query("UPDATE contents SET owner_id=$1, updated_at=now() WHERE id=$2 AND kind='app' AND deleted_at IS NULL").bind(owner).bind(app).execute(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    audit(&state, admin.user_id, "app.transfer", &app.to_string(), &json!({ "owner_id": owner.to_string() })).await?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
struct Grants { grants: Vec<Grant> }
#[derive(Debug, Deserialize)]
struct Grant { grantee: String, #[serde(default = "default_true")] can_open: bool }
fn default_true() -> bool { true }

async fn admin_grants(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>, Json(body): Json<Grants>) -> Result<Json<Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    let app = uuid_of(&id)?;
    let mut tx = state.pool.begin().await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query("DELETE FROM content_grants WHERE content_id=$1").bind(app).execute(&mut *tx).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    for grant in &body.grants {
        sqlx::query("INSERT INTO content_grants (content_id, grantee, can_install) VALUES ($1,$2,$3)").bind(app).bind(&grant.grantee).bind(grant.can_open).execute(&mut *tx).await.map_err(|_| error(StatusCode::BAD_REQUEST, "VALIDATION"))?;
    }
    let visibility = if body.grants.iter().any(|grant| grant.grantee == "all") { "all" } else { "selected" };
    sqlx::query("UPDATE contents SET visibility=$1 WHERE id=$2").bind(visibility).bind(app).execute(&mut *tx).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    tx.commit().await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    audit(&state, admin.user_id, "app.grants", &app.to_string(), &json!({})).await?;
    Ok(Json(json!({ "ok": true })))
}
