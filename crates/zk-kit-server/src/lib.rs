//! Shared server infrastructure (state, JWT, password, admin guard, audit chain).
#![forbid(unsafe_code)]

use std::time::{SystemTime, UNIX_EPOCH};

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use sqlx::Row;

/// Shared server state.
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL pool.
    pub pool: PgPool,
    /// JWT signing/verification material.
    pub jwt: JwtKeys,
}

impl AppState {
    /// Build state with a fresh ephemeral EdDSA key.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool, jwt: JwtKeys::ephemeral() }
    }
}

/// Error shape shared by server domain routes.
#[derive(Debug)]
pub struct ServerError {
    pub status: StatusCode,
    pub code: &'static str,
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "ok": false, "error": self.code }))).into_response()
    }
}

/// Build a server error.
#[must_use]
pub fn error(status: StatusCode, code: &'static str) -> ServerError {
    ServerError { status, code }
}

/// EdDSA JWT keys.
#[derive(Clone)]
pub struct JwtKeys {
    signing: SigningKey,
    kid: String,
}

impl JwtKeys {
    /// Generate ephemeral dev keys.
    #[must_use]
    pub fn ephemeral() -> Self {
        Self { signing: SigningKey::generate(&mut OsRng), kid: "ephemeral".to_owned() }
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

/// Current time seconds.
#[must_use]
pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Random URL-safe token.
#[must_use]
pub fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    rand_core::RngCore::fill_bytes(&mut OsRng, &mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// SHA-256 hex.
#[must_use]
pub fn sha256_hex(value: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(value.as_bytes());
    format!("{:x}", hash.finalize())
}

/// Hash a password.
pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default().hash_password(password.as_bytes(), &salt).map(|hash| hash.to_string()).map_err(|error| error.to_string())
}

/// Verify a password.
#[must_use]
pub fn verify_password(password: &str, encoded: &str) -> bool {
    PasswordHash::new(encoded).map(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()).unwrap_or(false)
}

/// Issue an access token (15m).
pub fn encode_token(state: &AppState, user_id: &str, role: &str, epoch: i32, device: &str) -> Result<String, ServerError> {
    let now = now_secs();
    let claims = Claims { sub: user_id.to_owned(), role: role.to_owned(), epoch, device: device.to_owned(), kid: state.jwt.kid.clone(), iat: now, exp: now.saturating_add(15 * 60) };
    let header = json!({ "alg": "EdDSA", "typ": "JWT", "kid": state.jwt.kid });
    let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "TOKEN_ERROR"))?);
    let payload_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "TOKEN_ERROR"))?);
    let message = format!("{header_b64}.{payload_b64}");
    let signature = state.jwt.signing.sign(message.as_bytes());
    Ok(format!("{message}.{}", URL_SAFE_NO_PAD.encode(signature.to_bytes())))
}

fn decode_token(state: &AppState, token: &str) -> Result<Claims, ServerError> {
    let mut parts = token.split('.');
    let (Some(header_b64), Some(payload_b64), Some(signature_b64), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return Err(error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"));
    };
    let signature_bytes = URL_SAFE_NO_PAD.decode(signature_b64).map_err(|_| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let signature = Signature::from_slice(&signature_bytes).map_err(|_| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    state.jwt.verifying().verify_strict(format!("{header_b64}.{payload_b64}").as_bytes(), &signature).map_err(|_| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let payload = URL_SAFE_NO_PAD.decode(payload_b64).map_err(|_| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let claims: Claims = serde_json::from_slice(&payload).map_err(|_| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    if claims.exp <= now_secs() || claims.kid != state.jwt.kid {
        return Err(error(StatusCode::UNAUTHORIZED, "AUTH_EXPIRED"));
    }
    Ok(claims)
}

/// Bearer token from headers.
#[must_use]
pub fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers.get(header::AUTHORIZATION).and_then(|value| value.to_str().ok()).and_then(|value| value.strip_prefix("Bearer "))
}

/// Cookie value by name.
#[must_use]
pub fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(header::COOKIE).and_then(|value| value.to_str().ok()).and_then(|cookies| {
        cookies.split(';').find_map(|cookie| {
            let (key, value) = cookie.trim().split_once('=')?;
            if key == name { Some(value) } else { None }
        })
    })
}

/// Verify the current employee access token and DB status/epoch.
pub async fn current_user(state: &AppState, headers: &HeaderMap) -> Result<(String, String, i32), ServerError> {
    let token = bearer(headers).ok_or_else(|| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let claims = decode_token(state, token)?;
    let user_id = uuid::Uuid::parse_str(&claims.sub).map_err(|_| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let row = sqlx::query("SELECT role, token_epoch, status FROM users WHERE id = $1 AND deleted_at IS NULL")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let db_role: String = row.try_get("role").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let epoch: i32 = row.try_get("token_epoch").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let status: String = row.try_get("status").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if epoch != claims.epoch || status != "active" {
        return Err(error(StatusCode::UNAUTHORIZED, "AUTH_EXPIRED"));
    }
    Ok((claims.sub, db_role, epoch))
}

/// Admin session context.
pub struct AdminContext {
    pub user_id: uuid::Uuid,
}

/// Verify admin cookie session; `write` enforces CSRF + super_admin.
pub async fn require_admin(state: &AppState, headers: &HeaderMap, write: bool) -> Result<AdminContext, ServerError> {
    let session = cookie(headers, "zk_admin").ok_or_else(|| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let row = sqlx::query(
        "SELECT s.id, s.csrf_hash, s.user_id, u.role, u.status FROM admin_sessions s JOIN users u ON u.id = s.user_id
         WHERE s.session_hash = $1 AND s.revoked_at IS NULL AND s.expires_at > now()",
    )
    .bind(sha256_hex(session))
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
    .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let status: String = row.try_get("status").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if status != "active" {
        return Err(error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"));
    }
    let role: String = row.try_get("role").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if role != "super_admin" && role != "auditor" {
        return Err(error(StatusCode::FORBIDDEN, "FORBIDDEN"));
    }
    if write {
        if role != "super_admin" {
            return Err(error(StatusCode::FORBIDDEN, "FORBIDDEN"));
        }
        let csrf_hash: String = row.try_get("csrf_hash").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
        let supplied = headers.get("x-csrf-token").and_then(|value| value.to_str().ok()).unwrap_or("");
        if sha256_hex(supplied) != csrf_hash {
            return Err(error(StatusCode::FORBIDDEN, "CSRF_INVALID"));
        }
    }
    Ok(AdminContext { user_id: row.try_get("user_id").map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))? })
}

/// Append an audit row with a hash chain.
pub async fn audit(state: &AppState, actor: uuid::Uuid, action: &str, target: &str, detail: &serde_json::Value) -> Result<(), ServerError> {
    let prev: String = sqlx::query_scalar("SELECT COALESCE((SELECT hash FROM audit_log ORDER BY id DESC LIMIT 1), '')")
        .fetch_one(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "AUDIT_FAILED"))?;
    let hash = sha256_hex(&format!("{prev}{action}{target}{detail}"));
    sqlx::query("INSERT INTO audit_log (actor_id, action, target, detail, prev_hash, hash) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(actor)
        .bind(action)
        .bind(target)
        .bind(detail)
        .bind(&prev)
        .bind(hash)
        .execute(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "AUDIT_FAILED"))?;
    Ok(())
}

/// Bootstrap the first super admin when the users table is empty.
pub async fn ensure_bootstrap_admin(state: &AppState, username: &str, password: &str) -> Result<bool, sqlx::Error> {
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
