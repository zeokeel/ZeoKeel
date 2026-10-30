//! M2 P1 local auth: password login, EdDSA JWT, refresh rotation, admin cookie + CSRF.
#![allow(clippy::too_many_lines)] // focused P1 auth slice

use std::time::{SystemTime, UNIX_EPOCH};

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::Row;

use crate::AppState;

const ACCESS_TTL_SECS: u64 = 15 * 60;
const ADMIN_TTL_SECS: u64 = 8 * 60 * 60;

/// JWT signing material (ephemeral when `ZEOKEEL_JWT_KEY` is unset).
#[derive(Clone)]
pub struct JwtKeys {
    signing: SigningKey,
    kid: String,
}

impl JwtKeys {
    /// Generate an ephemeral key pair (development); production supplies a persisted key.
    #[must_use]
    pub fn ephemeral() -> Self {
        let mut rng = OsRng;
        Self { signing: SigningKey::generate(&mut rng), kid: "ephemeral".to_owned() }
    }

    fn verifying(&self) -> VerifyingKey {
        self.signing.verifying_key()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    role: String,
    epoch: i32,
    device: String,
    kid: String,
    iat: u64,
    exp: u64,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub device_id: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
    pub device_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminLoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug)]
struct AuthError {
    status: StatusCode,
    code: &'static str,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "ok": false, "error": self.code }))).into_response()
    }
}

fn auth_error(status: StatusCode, code: &'static str) -> AuthError {
    AuthError { status, code }
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    rand_core::RngCore::fill_bytes(&mut OsRng, &mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn sha256_hex(value: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(value.as_bytes());
    format!("{:x}", hash.finalize())
}

/// Hash a password for storage (argon2id default).
pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| error.to_string())
}

fn verify_password(password: &str, encoded: &str) -> bool {
    PasswordHash::new(encoded)
        .map(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
        .unwrap_or(false)
}

/// Create the first super admin when the users table is empty.
pub async fn ensure_bootstrap_admin(
    state: &AppState,
    username: &str,
    password: &str,
) -> Result<bool, sqlx::Error> {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users").fetch_one(&state.pool).await?;
    if count > 0 {
        return Ok(false);
    }
    let hash = hash_password(password).map_err(sqlx::Error::Protocol)?;
    sqlx::query("INSERT INTO users (username, display_name, password_hash, role) VALUES ($1, $1, $2, 'super_admin')")
        .bind(username)
        .bind(hash)
        .execute(&state.pool)
        .await?;
    Ok(true)
}

fn encode_token(state: &AppState, user_id: &str, role: &str, epoch: i32, device: &str) -> Result<String, AuthError> {
    let now = now_secs();
    let claims = Claims {
        sub: user_id.to_owned(),
        role: role.to_owned(),
        epoch,
        device: device.to_owned(),
        kid: state.jwt.kid.clone(),
        iat: now,
        exp: now.saturating_add(ACCESS_TTL_SECS),
    };
    let header = json!({ "alg": "EdDSA", "typ": "JWT", "kid": state.jwt.kid });
    let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "TOKEN_ERROR"))?);
    let payload_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "TOKEN_ERROR"))?);
    let message = format!("{header_b64}.{payload_b64}");
    let signature = state.jwt.signing.sign(message.as_bytes());
    Ok(format!("{message}.{}", URL_SAFE_NO_PAD.encode(signature.to_bytes())))
}

fn decode_token(state: &AppState, token: &str) -> Result<Claims, AuthError> {
    let mut parts = token.split('.');
    let (Some(header_b64), Some(payload_b64), Some(signature_b64), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"));
    };
    let signature_bytes = URL_SAFE_NO_PAD
        .decode(signature_b64)
        .map_err(|_| auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    state
        .jwt
        .verifying()
        .verify_strict(format!("{header_b64}.{payload_b64}").as_bytes(), &signature)
        .map_err(|_| auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let payload = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let claims: Claims = serde_json::from_slice(&payload)
        .map_err(|_| auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    if claims.exp <= now_secs() || claims.kid != state.jwt.kid {
        return Err(auth_error(StatusCode::UNAUTHORIZED, "AUTH_EXPIRED"));
    }
    Ok(claims)
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
}

fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(header::COOKIE).and_then(|value| value.to_str().ok()).and_then(|cookies| {
        cookies.split(';').find_map(|cookie| {
            let (key, value) = cookie.trim().split_once('=')?;
            if key == name { Some(value) } else { None }
        })
    })
}

async fn current_user(state: &AppState, headers: &HeaderMap) -> Result<(String, String, i32), AuthError> {
    let token = bearer(headers).ok_or_else(|| auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let claims = decode_token(state, token)?;
    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let row = sqlx::query("SELECT role, token_epoch, status FROM users WHERE id = $1 AND deleted_at IS NULL")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let db_role: String = row.try_get("role").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let epoch: i32 = row.try_get("token_epoch").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let status: String = row.try_get("status").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if epoch != claims.epoch || status != "active" {
        return Err(auth_error(StatusCode::UNAUTHORIZED, "AUTH_EXPIRED"));
    }
    Ok((claims.sub, db_role, epoch))
}

async fn methods() -> Json<serde_json::Value> {
    Json(json!({ "mode": "local", "providers": [], "min_client_version": "0.1.0" }))
}

async fn login(State(state): State<AppState>, Json(request): Json<LoginRequest>) -> Result<Json<serde_json::Value>, AuthError> {
    if request.device_id.trim().is_empty() {
        return Err(auth_error(StatusCode::BAD_REQUEST, "DEVICE_REQUIRED"));
    }
    let row = sqlx::query("SELECT id, role, token_epoch, password_hash, status FROM users WHERE username = $1 AND deleted_at IS NULL")
        .bind(&request.username)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| auth_error(StatusCode::UNAUTHORIZED, "AUTH_FAILED"))?;
    let status: String = row.try_get("status").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if status != "active" {
        return Err(auth_error(StatusCode::FORBIDDEN, "ACCOUNT_DISABLED"));
    }
    let stored: Option<String> = row.try_get("password_hash").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let Some(stored) = stored else {
        return Err(auth_error(StatusCode::UNAUTHORIZED, "AUTH_FAILED"));
    };
    if !verify_password(&request.password, &stored) {
        return Err(auth_error(StatusCode::UNAUTHORIZED, "AUTH_FAILED"));
    }
    let user_id: uuid::Uuid = row.try_get("id").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let role: String = row.try_get("role").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let epoch: i32 = row.try_get("token_epoch").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let user_id = user_id.to_string();
    let access = encode_token(&state, &user_id, &role, epoch, &request.device_id)?;
    let refresh = random_token();
    sqlx::query("INSERT INTO refresh_tokens (user_id, device_id, token_hash, expires_at) VALUES ($1, $2, $3, now() + interval '30 days')")
        .bind(uuid::Uuid::parse_str(&user_id).map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?)
        .bind(&request.device_id)
        .bind(sha256_hex(&refresh))
        .execute(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query("INSERT INTO user_devices (user_id, device_id, platform) VALUES ($1, $2, 'unknown') ON CONFLICT (user_id, device_id) DO UPDATE SET last_seen_at = now()")
        .bind(uuid::Uuid::parse_str(&user_id).map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?)
        .bind(&request.device_id)
        .execute(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(Json(json!({
        "ok": true,
        "access_token": access,
        "refresh_token": refresh,
        "expires_in": ACCESS_TTL_SECS,
        "user": { "id": user_id, "role": role, "username": request.username },
    })))
}

async fn refresh(State(state): State<AppState>, Json(request): Json<RefreshRequest>) -> Result<Json<serde_json::Value>, AuthError> {
    let digest = sha256_hex(&request.refresh_token);
    let row = sqlx::query("SELECT id, user_id, device_id, expires_at, revoked_at FROM refresh_tokens WHERE token_hash = $1")
        .bind(&digest)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| auth_error(StatusCode::UNAUTHORIZED, "AUTH_EXPIRED"))?;
    let revoked: Option<chrono::DateTime<chrono::Utc>> = row.try_get("revoked_at").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if revoked.is_some() {
        return Err(auth_error(StatusCode::UNAUTHORIZED, "REFRESH_REUSED"));
    }
    let device: String = row.try_get("device_id").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if device != request.device_id {
        return Err(auth_error(StatusCode::UNAUTHORIZED, "DEVICE_MISMATCH"));
    }
    let user_id: uuid::Uuid = row.try_get("user_id").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let user = sqlx::query("SELECT role, token_epoch, status FROM users WHERE id = $1 AND deleted_at IS NULL")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| auth_error(StatusCode::UNAUTHORIZED, "AUTH_EXPIRED"))?;
    let status: String = user.try_get("status").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if status != "active" {
        return Err(auth_error(StatusCode::FORBIDDEN, "ACCOUNT_DISABLED"));
    }
    let role: String = user.try_get("role").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let epoch: i32 = user.try_get("token_epoch").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let next = random_token();
    sqlx::query("UPDATE refresh_tokens SET revoked_at = now() WHERE id = $1")
        .bind(row.try_get::<uuid::Uuid, _>("id").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?)
        .execute(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query("INSERT INTO refresh_tokens (user_id, device_id, token_hash, expires_at) VALUES ($1, $2, $3, now() + interval '30 days')")
        .bind(user_id)
        .bind(&request.device_id)
        .bind(sha256_hex(&next))
        .execute(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let user_id = user_id.to_string();
    let access = encode_token(&state, &user_id, &role, epoch, &request.device_id)?;
    Ok(Json(json!({ "ok": true, "access_token": access, "refresh_token": next, "expires_in": ACCESS_TTL_SECS })))
}

async fn me(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<serde_json::Value>, AuthError> {
    let (user_id, role, _) = current_user(&state, &headers).await?;
    Ok(Json(json!({ "id": user_id, "role": role, "permissions": [] })))
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<serde_json::Value>, AuthError> {
    let (user_id, _, _) = current_user(&state, &headers).await?;
    sqlx::query("UPDATE refresh_tokens SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(uuid::Uuid::parse_str(&user_id).map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?)
        .execute(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(Json(json!({ "ok": true })))
}

async fn admin_login(State(state): State<AppState>, Json(request): Json<AdminLoginRequest>) -> Result<Response, AuthError> {
    let row = sqlx::query("SELECT id, password_hash FROM users WHERE username = $1 AND role = 'super_admin' AND status = 'active' AND deleted_at IS NULL")
        .bind(&request.username)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| auth_error(StatusCode::UNAUTHORIZED, "AUTH_FAILED"))?;
    let stored: Option<String> = row.try_get("password_hash").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let Some(stored) = stored else {
        return Err(auth_error(StatusCode::UNAUTHORIZED, "AUTH_FAILED"));
    };
    if !verify_password(&request.password, &stored) {
        return Err(auth_error(StatusCode::UNAUTHORIZED, "AUTH_FAILED"));
    }
    let user_id: uuid::Uuid = row.try_get("id").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let session = random_token();
    let csrf = random_token();
    sqlx::query("INSERT INTO admin_sessions (user_id, session_hash, csrf_hash, expires_at) VALUES ($1, $2, $3, now() + interval '8 hours')")
        .bind(user_id)
        .bind(sha256_hex(&session))
        .bind(sha256_hex(&csrf))
        .execute(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let secure = std::env::var("ZEOKEEL_COOKIE_SECURE").is_ok_and(|value| value == "1");
    let cookie = format!("zk_admin={session}; HttpOnly; SameSite=Lax; Path=/{}", if secure { "; Secure" } else { "" });
    let mut response = Json(json!({ "ok": true, "csrf_token": csrf, "expires_in": ADMIN_TTL_SECS })).into_response();
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    Ok(response)
}

async fn admin_logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, AuthError> {
    let Some(session) = cookie(&headers, "zk_admin") else {
        return Err(auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"));
    };
    let row = sqlx::query("SELECT id, csrf_hash FROM admin_sessions WHERE session_hash = $1 AND revoked_at IS NULL AND expires_at > now()")
        .bind(sha256_hex(session))
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let csrf_hash: String = row.try_get("csrf_hash").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let supplied = headers.get("x-csrf-token").and_then(|value| value.to_str().ok()).unwrap_or("");
    if sha256_hex(supplied) != csrf_hash {
        return Err(auth_error(StatusCode::FORBIDDEN, "CSRF_INVALID"));
    }
    sqlx::query("UPDATE admin_sessions SET revoked_at = now() WHERE id = $1")
        .bind(row.try_get::<uuid::Uuid, _>("id").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?)
        .execute(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let mut response = Json(json!({ "ok": true })).into_response();
    response.headers_mut().insert(header::SET_COOKIE, HeaderValue::from_static("zk_admin=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0"));
    Ok(response)
}

/// Auth routes (client + admin login/session).
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)] // test-only
mod tests {
    use super::*;

    #[test]
    fn jwt_roundtrip_and_tamper() {
        let state = AppState::new_for_test();
        let token = encode_token(&state, "u1", "employee", 1, "d1").unwrap();
        let claims = decode_token(&state, &token).unwrap();
        assert_eq!(claims.sub, "u1");
        assert_eq!(claims.device, "d1");
        let mut tampered = token.clone();
        tampered.push('x');
        assert!(decode_token(&state, &tampered).is_err());
    }

    #[test]
    fn password_hash_verifies() {
        let hash = hash_password("correct horse").unwrap();
        assert!(verify_password("correct horse", &hash));
        assert!(!verify_password("wrong", &hash));
    }
}
