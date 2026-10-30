//! `srv.auth.session`: local login, EdDSA JWT, refresh rotation, admin cookie + CSRF.
#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)] // focused auth domain



use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use sqlx::Row;

use zk_kit_server::{
    error, encode_token, random_token, require_admin, sha256_hex, verify_password,
    AppState, ServerError,
};

const ADMIN_TTL_SECS: u64 = 8 * 60 * 60;
const ACCESS_TTL_SECS: u64 = 15 * 60;

use zk_contract_auth::{AdminLoginRequest, LoginRequest, RefreshRequest};

async fn methods() -> Json<serde_json::Value> {
    Json(json!({ "mode": "local", "providers": [], "min_client_version": "0.1.0" }))
}

async fn login(State(state): State<AppState>, Json(request): Json<LoginRequest>) -> Result<Json<serde_json::Value>, ServerError> {
    if request.device_id.trim().is_empty() {
        return Err(error(StatusCode::BAD_REQUEST, "DEVICE_REQUIRED"));
    }
    let row = sqlx::query("SELECT id, role, token_epoch, password_hash, status FROM users WHERE username = $1 AND deleted_at IS NULL")
        .bind(&request.username)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "AUTH_FAILED"))?;
    let status: String = row.try_get("status").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if status != "active" {
        return Err(error(StatusCode::FORBIDDEN, "ACCOUNT_DISABLED"));
    }
    let stored: Option<String> = row.try_get("password_hash").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let Some(stored) = stored else {
        return Err(error(StatusCode::UNAUTHORIZED, "AUTH_FAILED"));
    };
    if !verify_password(&request.password, &stored) {
        return Err(error(StatusCode::UNAUTHORIZED, "AUTH_FAILED"));
    }
    let user_id: uuid::Uuid = row.try_get("id").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let role: String = row.try_get("role").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let epoch: i32 = row.try_get("token_epoch").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let user_id_string = user_id.to_string();
    let access = encode_token(&state, &user_id_string, &role, epoch, &request.device_id)?;
    let refresh = random_token();
    sqlx::query("INSERT INTO refresh_tokens (user_id, device_id, token_hash, expires_at) VALUES ($1, $2, $3, now() + interval '30 days')")
        .bind(user_id)
        .bind(&request.device_id)
        .bind(sha256_hex(&refresh))
        .execute(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query("INSERT INTO user_devices (user_id, device_id, platform) VALUES ($1, $2, 'unknown') ON CONFLICT (user_id, device_id) DO UPDATE SET last_seen_at = now()")
        .bind(user_id)
        .bind(&request.device_id)
        .execute(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(Json(json!({
        "ok": true,
        "access_token": access,
        "refresh_token": refresh,
        "expires_in": ACCESS_TTL_SECS,
        "user": { "id": user_id_string, "role": role, "username": request.username },
    })))
}

async fn refresh(State(state): State<AppState>, Json(request): Json<RefreshRequest>) -> Result<Json<serde_json::Value>, ServerError> {
    let digest = sha256_hex(&request.refresh_token);
    let row = sqlx::query("SELECT id, user_id, device_id, revoked_at FROM refresh_tokens WHERE token_hash = $1")
        .bind(&digest)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "AUTH_EXPIRED"))?;
    let revoked: Option<chrono::DateTime<chrono::Utc>> = row.try_get("revoked_at").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if revoked.is_some() {
        return Err(error(StatusCode::UNAUTHORIZED, "REFRESH_REUSED"));
    }
    let device: String = row.try_get("device_id").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if device != request.device_id {
        return Err(error(StatusCode::UNAUTHORIZED, "DEVICE_MISMATCH"));
    }
    let user_id: uuid::Uuid = row.try_get("user_id").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let user = sqlx::query("SELECT role, token_epoch, status FROM users WHERE id = $1 AND deleted_at IS NULL")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "AUTH_EXPIRED"))?;
    let status: String = user.try_get("status").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if status != "active" {
        return Err(error(StatusCode::FORBIDDEN, "ACCOUNT_DISABLED"));
    }
    let role: String = user.try_get("role").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let epoch: i32 = user.try_get("token_epoch").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let next = random_token();
    sqlx::query("UPDATE refresh_tokens SET revoked_at = now() WHERE id = $1")
        .bind(row.try_get::<uuid::Uuid, _>("id").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?)
        .execute(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query("INSERT INTO refresh_tokens (user_id, device_id, token_hash, expires_at) VALUES ($1, $2, $3, now() + interval '30 days')")
        .bind(user_id)
        .bind(&request.device_id)
        .bind(sha256_hex(&next))
        .execute(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let access = encode_token(&state, &user_id.to_string(), &role, epoch, &request.device_id)?;
    Ok(Json(json!({ "ok": true, "access_token": access, "refresh_token": next, "expires_in": ACCESS_TTL_SECS })))
}

async fn me(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<serde_json::Value>, ServerError> {
    let (user_id, role, _) = zk_kit_server::current_user(&state, &headers).await?;
    Ok(Json(json!({ "id": user_id, "role": role, "permissions": [] })))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<serde_json::Value>, ServerError> {
    let (user_id, _, _) = zk_kit_server::current_user(&state, &headers).await?;
    sqlx::query("UPDATE refresh_tokens SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(uuid::Uuid::parse_str(&user_id).map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?)
        .execute(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(Json(json!({ "ok": true })))
}

async fn admin_login(State(state): State<AppState>, Json(request): Json<AdminLoginRequest>) -> Result<Response, ServerError> {
    let row = sqlx::query("SELECT id, password_hash FROM users WHERE username = $1 AND role IN ('super_admin', 'auditor') AND status = 'active' AND deleted_at IS NULL")
        .bind(&request.username)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "AUTH_FAILED"))?;
    let stored: Option<String> = row.try_get("password_hash").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let Some(stored) = stored else {
        return Err(error(StatusCode::UNAUTHORIZED, "AUTH_FAILED"));
    };
    if !verify_password(&request.password, &stored) {
        return Err(error(StatusCode::UNAUTHORIZED, "AUTH_FAILED"));
    }
    let user_id: uuid::Uuid = row.try_get("id").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let session = random_token();
    let csrf = random_token();
    sqlx::query("INSERT INTO admin_sessions (user_id, session_hash, csrf_hash, expires_at) VALUES ($1, $2, $3, now() + interval '8 hours')")
        .bind(user_id)
        .bind(sha256_hex(&session))
        .bind(sha256_hex(&csrf))
        .execute(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let secure = std::env::var("ZEOKEEL_COOKIE_SECURE").is_ok_and(|value| value == "1");
    let cookie = format!("zk_admin={session}; HttpOnly; SameSite=Lax; Path=/{}", if secure { "; Secure" } else { "" });
    let mut response = Json(json!({ "ok": true, "csrf_token": csrf, "expires_in": ADMIN_TTL_SECS })).into_response();
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    Ok(response)
}

async fn admin_logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, ServerError> {
    require_admin(&state, &headers, true).await?;
    let Some(session) = zk_kit_server::cookie(&headers, "zk_admin") else {
        return Err(error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"));
    };
    sqlx::query("UPDATE admin_sessions SET revoked_at = now() WHERE session_hash = $1")
        .bind(sha256_hex(session))
        .execute(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let mut response = Json(json!({ "ok": true })).into_response();
    response.headers_mut().insert(header::SET_COOKIE, HeaderValue::from_static("zk_admin=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0"));
    Ok(response)
}

/// Auth routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/client/v1/auth/methods", get(methods))
        .route("/api/client/v1/auth/login", post(login))
        .route("/api/client/v1/auth/refresh", post(refresh))
        .route("/api/client/v1/auth/logout", post(logout))
        .route("/api/client/v1/auth/me", get(me))
        .route("/api/admin/auth/login", post(admin_login))
        .route("/api/admin/auth/logout", post(admin_logout))
}

/// Re-export bootstrap helper for the composition root.
pub use zk_kit_server::ensure_bootstrap_admin;

/// Immutable migrations owned by this domain.
pub const MIGRATIONS: &[(&str, &str)] = &[("0002_auth", include_str!("../migrations/0002_auth.sql"))];
