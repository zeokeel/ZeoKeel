//! Shared server infrastructure (state, JWT, password, admin guard, audit chain).
//!
//! @capability kit.server
#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::future::Future;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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
        Self {
            pool,
            jwt: JwtKeys::ephemeral(),
        }
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
        (
            self.status,
            Json(json!({ "ok": false, "error": self.code })),
        )
            .into_response()
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
        Self {
            signing: SigningKey::generate(&mut OsRng),
            kid: "ephemeral".to_owned(),
        }
    }

    fn verifying(&self) -> VerifyingKey {
        self.signing.verifying_key()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    aud: String,
    sub: String,
    uid: String,
    role: String,
    epoch: i32,
    device: String,
    channel: String,
    jti: String,
    kid: String,
    iat: u64,
    exp: u64,
}

/// Access-token issuer (`ZEOKEEL_JWT_ISSUER`).
#[must_use]
pub fn jwt_issuer() -> String {
    std::env::var("ZEOKEEL_JWT_ISSUER").unwrap_or_else(|_| "zeokeel".to_owned())
}

/// Access-token audience (`ZEOKEEL_JWT_AUDIENCE`).
#[must_use]
pub fn jwt_audience() -> String {
    std::env::var("ZEOKEEL_JWT_AUDIENCE").unwrap_or_else(|_| "zeokeel-client".to_owned())
}

/// Current time seconds.
#[must_use]
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
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
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| error.to_string())
}

/// Verify a password.
#[must_use]
pub fn verify_password(password: &str, encoded: &str) -> bool {
    PasswordHash::new(encoded)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

/// Issue an access token (15m).
pub fn encode_token(
    state: &AppState,
    user_id: &str,
    role: &str,
    epoch: i32,
    device: &str,
    channel: &str,
) -> Result<String, ServerError> {
    let now = now_secs();
    let claims = Claims {
        iss: jwt_issuer(),
        aud: jwt_audience(),
        sub: user_id.to_owned(),
        uid: user_id.to_owned(),
        role: role.to_owned(),
        epoch,
        device: device.to_owned(),
        channel: channel.to_owned(),
        jti: random_token(),
        kid: state.jwt.kid.clone(),
        iat: now,
        exp: now.saturating_add(15 * 60),
    };
    let header = json!({ "alg": "EdDSA", "typ": "JWT", "kid": state.jwt.kid });
    let header_b64 = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&header)
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "TOKEN_ERROR"))?,
    );
    let payload_b64 = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&claims)
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "TOKEN_ERROR"))?,
    );
    let message = format!("{header_b64}.{payload_b64}");
    let signature = state.jwt.signing.sign(message.as_bytes());
    Ok(format!(
        "{message}.{}",
        URL_SAFE_NO_PAD.encode(signature.to_bytes())
    ))
}

fn decode_token(state: &AppState, token: &str) -> Result<Claims, ServerError> {
    let mut parts = token.split('.');
    let (Some(header_b64), Some(payload_b64), Some(signature_b64), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"));
    };
    let signature_bytes = URL_SAFE_NO_PAD
        .decode(signature_b64)
        .map_err(|_| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    state
        .jwt
        .verifying()
        .verify_strict(format!("{header_b64}.{payload_b64}").as_bytes(), &signature)
        .map_err(|_| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let payload = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let claims: Claims = serde_json::from_slice(&payload)
        .map_err(|_| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    if claims.exp <= now_secs() || claims.kid != state.jwt.kid {
        return Err(error(StatusCode::UNAUTHORIZED, "AUTH_EXPIRED"));
    }
    if claims.iss != jwt_issuer() || claims.aud != jwt_audience() || claims.uid != claims.sub {
        return Err(error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"));
    }
    Ok(claims)
}

/// Bearer token from headers.
#[must_use]
pub fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
}

/// Cookie value by name.
#[must_use]
pub fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|cookie| {
                let (key, value) = cookie.trim().split_once('=')?;
                if key == name {
                    Some(value)
                } else {
                    None
                }
            })
        })
}

/// Verify the current employee access token and DB status/epoch.
pub async fn current_user(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(String, String, i32), ServerError> {
    let token = bearer(headers).ok_or_else(|| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let claims = decode_token(state, token)?;
    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map_err(|_| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let row = sqlx::query(
        "SELECT role, token_epoch, status FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
    .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let db_role: String = row
        .try_get("role")
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let epoch: i32 = row
        .try_get("token_epoch")
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let status: String = row
        .try_get("status")
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if epoch != claims.epoch || status != "active" {
        return Err(error(StatusCode::UNAUTHORIZED, "AUTH_EXPIRED"));
    }
    Ok((claims.sub, db_role, epoch))
}

/// Constant-time string comparison (CSRF token hashes).
///
/// Length differences still leak (unavoidable while hashing hex strings), but the
/// content comparison does not short-circuit.
#[must_use]
pub fn constant_time_eq(left: &str, right: &str) -> bool {
    let mut diff = u8::from(left.len() != right.len());
    let max = left.len().max(right.len());
    for index in 0..max {
        let a = left.as_bytes().get(index).copied().unwrap_or(0);
        let b = right.as_bytes().get(index).copied().unwrap_or(0);
        diff |= a ^ b;
    }
    diff == 0
}

/// Documented permission points for the employee role (`docs/04` §3).
pub const EMPLOYEE_PERMISSIONS: &[&str] = &[
    "auth:self",
    "config:read",
    "gateway:chat",
    "skills:read",
    "skills:publish",
    "connectors:use",
    "apps:use",
    "telemetry:write",
    "audit:self",
    "wallet:self",
];

/// Read-only admin surfaces available to the auditor role.
pub const AUDITOR_PERMISSIONS: &[&str] = &[
    "auth:self",
    "config:read",
    "audit:self",
    "admin:audit:read",
    "admin:usage:read",
    "admin:users:read",
    "admin:jobs:read",
];

/// Full admin surfaces for the super admin role.
pub const SUPER_ADMIN_PERMISSIONS: &[&str] = &[
    "auth:self",
    "config:read",
    "gateway:chat",
    "skills:read",
    "skills:publish",
    "connectors:use",
    "apps:use",
    "telemetry:write",
    "audit:self",
    "wallet:self",
    "admin:users",
    "admin:wallet",
    "admin:pricing",
    "admin:providers",
    "admin:models",
    "admin:skills",
    "admin:connectors",
    "admin:apps",
    "admin:audit",
    "admin:usage",
    "admin:settings",
    "admin:jobs",
];

/// Permission points for a role (unknown roles degrade to employee).
#[must_use]
pub fn permissions_for_role(role: &str) -> Vec<&'static str> {
    match role {
        "super_admin" => SUPER_ADMIN_PERMISSIONS.to_vec(),
        "auditor" => AUDITOR_PERMISSIONS.to_vec(),
        _ => EMPLOYEE_PERMISSIONS.to_vec(),
    }
}

/// Default minimum supported client version when no environment override exists.
pub const DEFAULT_MIN_CLIENT_VERSION: &str = "0.1.0";

/// Minimum supported client version (`ZEOKEEL_MIN_CLIENT_VERSION`, validated).
#[must_use]
pub fn min_client_version() -> String {
    std::env::var("ZEOKEEL_MIN_CLIENT_VERSION")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| parse_semver(value).is_some())
        .unwrap_or_else(|| DEFAULT_MIN_CLIENT_VERSION.to_owned())
}

/// Parse a plain `major.minor.patch` version (pre-release suffix ignored).
#[must_use]
pub fn parse_semver(value: &str) -> Option<(u32, u32, u32)> {
    let trimmed = value.trim();
    let mut parts = trimmed.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next()?.parse::<u32>().ok()?;
    let patch = parts.next()?.split('-').next()?.parse::<u32>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// Client semver from `X-ZeoKeel-Client: <semver>;<channel>;<platform>;<device>`.
#[must_use]
pub fn client_version(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get("x-zeokeel-client")?.to_str().ok()?;
    raw.split(';')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// Channel from `X-ZeoKeel-Client: <semver>;<channel>;<platform>;<device>`.
#[must_use]
pub fn client_channel(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get("x-zeokeel-client")?.to_str().ok()?;
    raw.split(';')
        .nth(1)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// Environment key for a channel-specific minimum client version.
///
/// `official` → `ZEOKEEL_MIN_CLIENT_VERSION_OFFICIAL`, `oem-acme` →
/// `ZEOKEEL_MIN_CLIENT_VERSION_OEM_ACME`.
#[must_use]
pub fn channel_min_version_env_key(channel: &str) -> String {
    let suffix: String = channel
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() {
                value.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("ZEOKEEL_MIN_CLIENT_VERSION_{suffix}")
}

/// Authoritative server/build channel (compiled brand), never the client header.
#[must_use]
pub fn server_channel() -> String {
    brand_snapshot()
        .get("channel")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("official")
        .to_owned()
}

/// Minimum client version for the authoritative server channel.
///
/// Precedence: channel-specific env (`ZEOKEEL_MIN_CLIENT_VERSION_<CHANNEL>`,
/// using [`server_channel`]) → global `ZEOKEEL_MIN_CLIENT_VERSION` →
/// [`DEFAULT_MIN_CLIENT_VERSION`]. The client header channel is diagnostic only
/// and must never choose a lower minimum.
#[must_use]
pub fn min_client_version_for_server() -> String {
    min_client_version_for_channel(Some(&server_channel()))
}

/// Minimum client version for an explicit channel (env override → global).
///
/// Kept explicit for tests/ops tooling; the HTTP gate must call
/// [`min_client_version_for_server`], not this function with client input.
#[must_use]
pub fn min_client_version_for_channel(channel: Option<&str>) -> String {
    if let Some(channel) = channel.map(str::trim).filter(|value| !value.is_empty()) {
        if let Ok(value) = std::env::var(channel_min_version_env_key(channel)) {
            let value = value.trim().to_owned();
            if parse_semver(&value).is_some() {
                return value;
            }
        }
    }
    min_client_version()
}

/// Whether the request carries a client version below the configured minimum.
///
/// An absent or malformed client header is allowed (login/first-run compatibility);
/// stable surfaces must remain reachable so old clients can discover updates.
#[must_use]
pub fn client_is_too_old(headers: &HeaderMap) -> bool {
    let Some(current) = client_version(headers).and_then(|value| parse_semver(&value)) else {
        return false;
    };
    let minimum = parse_semver(&min_client_version_for_server()).unwrap_or((0, 0, 0));
    current < minimum
}

/// Read the request id from `X-Request-Id`, generating a UUID when missing/invalid.
#[must_use]
pub fn request_id_from_headers(headers: &HeaderMap) -> String {
    headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .map(str::to_owned)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

/// Attach the response `X-Request-Id` when not already present.
pub fn attach_request_id(response: &mut Response, request_id: &str) {
    if response.headers().get("x-request-id").is_none() {
        if let Ok(value) = HeaderValue::from_str(request_id) {
            response
                .headers_mut()
                .insert(HeaderName::from_static("x-request-id"), value);
        }
    }
}

tokio::task_local! {
    static REQUEST_ID: String;
}

/// Run a handler future with the request-scoped correlation id available to
/// [`audit_tx`] and other shared infrastructure.
pub async fn with_request_id<F>(request_id: String, future: F) -> F::Output
where
    F: Future,
{
    REQUEST_ID.scope(request_id, future).await
}

/// Request id for the current HTTP request, if the middleware installed it.
#[must_use]
pub fn current_request_id() -> Option<String> {
    REQUEST_ID.try_with(Clone::clone).ok()
}

/// Canonical nested 426 body shared by the HTTP gate, bootstrap and updates.
#[must_use]
pub fn upgrade_required_body(
    request_id: &str,
    current: Option<&str>,
    minimum: &str,
    message: &str,
) -> Value {
    json!({
        "error": {
            "code": "CLIENT_TOO_OLD",
            "message": message,
            "retryable": false,
            "request_id": request_id,
            "current": current,
            "minimum": minimum,
            "portal_url": "/portal",
        }
    })
}

/// Build the nested 426 response with the request-id echo header.
#[must_use]
pub fn upgrade_required_response(
    request_id: &str,
    current: Option<&str>,
    minimum: &str,
    message: &str,
) -> Response {
    let mut response = (
        StatusCode::UPGRADE_REQUIRED,
        Json(upgrade_required_body(request_id, current, minimum, message)),
    )
        .into_response();
    attach_request_id(&mut response, request_id);
    response
}

/// Stable ETag for a JSON payload (`"<sha256 hex>"`).
#[must_use]
pub fn content_etag(payload: &Value) -> String {
    let rendered = serde_json::to_vec(payload).unwrap_or_default();
    let mut hash = Sha256::new();
    hash.update(&rendered);
    format!("\"{:x}\"", hash.finalize())
}

/// Whether `If-None-Match` matches `etag` (including `*`).
#[must_use]
pub fn if_none_match(headers: &HeaderMap, etag: &str) -> bool {
    let candidate = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    candidate == etag || candidate == "*"
}

/// Compile-time channel brand snapshot (`shared/contracts/brand.json`).
///
/// `zk compose` regenerates the file for the selected channel before the server
/// binary is built, so OEM builds embed their own brand here.
#[must_use]
pub fn brand_snapshot() -> &'static Value {
    static BRAND: OnceLock<Value> = OnceLock::new();
    BRAND.get_or_init(|| {
        serde_json::from_str(include_str!("../../../shared/contracts/brand.json"))
            .unwrap_or_else(|_| json!({}))
    })
}

/// Read one JSON `system_settings` value (hot config).
pub async fn setting_value(
    pool: &sqlx::PgPool,
    key: &str,
) -> Result<Option<serde_json::Value>, ServerError> {
    sqlx::query_scalar("SELECT value FROM system_settings WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))
}

/// Read one integer `system_settings` value with a default.
pub async fn setting_i64(pool: &sqlx::PgPool, key: &str, default: i64) -> Result<i64, ServerError> {
    let Some(value) = setting_value(pool, key).await? else {
        return Ok(default);
    };
    match value {
        serde_json::Value::Number(number) => Ok(number.as_i64().unwrap_or(default)),
        serde_json::Value::String(text) => Ok(text.parse::<i64>().unwrap_or(default)),
        _ => Ok(default),
    }
}

/// Admin session limits from `system_settings` (docs/04 §3: 12h max, 60m idle).
pub async fn admin_session_limits(pool: &sqlx::PgPool) -> Result<(i64, i64), ServerError> {
    let ttl = setting_i64(pool, "admin.session_ttl_secs", 12 * 60 * 60)
        .await?
        .clamp(300, 12 * 60 * 60);
    let idle = setting_i64(pool, "admin.session_idle_secs", 60 * 60)
        .await?
        .clamp(60, 12 * 60 * 60);
    Ok((ttl, idle))
}

/// Admin session context.
pub struct AdminContext {
    pub user_id: uuid::Uuid,
}

/// Verify admin cookie session; `write` enforces CSRF + super_admin.
pub async fn require_admin(
    state: &AppState,
    headers: &HeaderMap,
    write: bool,
) -> Result<AdminContext, ServerError> {
    let session = cookie(headers, "zk_admin")
        .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let session_hash = sha256_hex(session);
    let (_, idle_secs) = admin_session_limits(&state.pool).await?;
    let row = sqlx::query(
        "SELECT s.id, s.csrf_hash, s.user_id, u.role, u.status FROM admin_sessions s JOIN users u ON u.id = s.user_id
         WHERE s.session_hash = $1 AND s.revoked_at IS NULL AND s.expires_at > now()
           AND s.last_seen_at > now() - ($2 * interval '1 second')",
    )
    .bind(&session_hash)
    .bind(idle_secs)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
    .ok_or_else(|| error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let status: String = row
        .try_get("status")
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if status != "active" {
        return Err(error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"));
    }
    let role: String = row
        .try_get("role")
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if role != "super_admin" && role != "auditor" {
        return Err(error(StatusCode::FORBIDDEN, "FORBIDDEN"));
    }
    if write {
        if role != "super_admin" {
            return Err(error(StatusCode::FORBIDDEN, "FORBIDDEN"));
        }
        let csrf_hash: String = row
            .try_get("csrf_hash")
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
        let supplied = headers
            .get("x-csrf-token")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");
        if !constant_time_eq(&sha256_hex(supplied), &csrf_hash) {
            return Err(error(StatusCode::FORBIDDEN, "CSRF_INVALID"));
        }
    }
    // Sliding idle window: a successfully validated admin request refreshes it.
    sqlx::query("UPDATE admin_sessions SET last_seen_at = now() WHERE session_hash = $1")
        .bind(&session_hash)
        .execute(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(AdminContext {
        user_id: row
            .try_get("user_id")
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?,
    })
}

/// Canonical JSON text with recursively sorted object keys.
///
/// `audit_log.detail` is `jsonb`, whose textual key order is normalization-
/// defined; hashing a raw `Display` form would make the chain order-dependent.
/// Insert and verify both use this canonical form so the hash is stable.
#[must_use]
pub fn canonical_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_unstable();
            let mut parts = Vec::with_capacity(keys.len());
            for key in keys {
                if let Some(inner) = map.get(key) {
                    let encoded_key = serde_json::to_string(key).unwrap_or_default();
                    parts.push(format!("{encoded_key}:{}", canonical_json(inner)));
                }
            }
            format!("{{{}}}", parts.join(","))
        }
        serde_json::Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(canonical_json).collect();
            format!("[{}]", parts.join(","))
        }
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

/// Advisory-lock slot serializing audit hash-chain appends.
const AUDIT_CHAIN_LOCK: i64 = 0x5A4B_4155_4449_5453;

/// Append an audit row with a hash chain inside a caller-owned transaction.
///
/// Fail-closed contract (`docs/07` §8, `docs/22` §8): callers must invoke this
/// before committing the business mutation, so an audit failure aborts both the
/// mutation and the audit row. The transaction-scoped advisory lock serializes
/// `prev_hash` reads so concurrent writers cannot fork the chain.
pub async fn audit_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: uuid::Uuid,
    action: &str,
    target: &str,
    detail: &serde_json::Value,
) -> Result<(), ServerError> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(AUDIT_CHAIN_LOCK)
        .execute(&mut **tx)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "AUDIT_FAILED"))?;
    let prev: String = sqlx::query_scalar(
        "SELECT COALESCE((SELECT hash FROM audit_log ORDER BY id DESC LIMIT 1), '')",
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "AUDIT_FAILED"))?;
    let hash = sha256_hex(&format!("{prev}{action}{target}{}", canonical_json(detail)));
    let request_id = current_request_id().unwrap_or_default();
    sqlx::query("INSERT INTO audit_log (actor_id, action, target, detail, prev_hash, hash, request_id) VALUES ($1, $2, $3, $4, $5, $6, $7)")
        .bind(actor)
        .bind(action)
        .bind(target)
        .bind(detail)
        .bind(&prev)
        .bind(hash)
        .bind(&request_id)
        .execute(&mut **tx)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "AUDIT_FAILED"))?;
    Ok(())
}

/// Append an audit row in its own transaction.
///
/// Retained for read-mostly domains; money and identity mutations must use
/// [`audit_tx`] inside their business transaction (fail-closed).
pub async fn audit(
    state: &AppState,
    actor: uuid::Uuid,
    action: &str,
    target: &str,
    detail: &serde_json::Value,
) -> Result<(), ServerError> {
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "AUDIT_FAILED"))?;
    audit_tx(&mut tx, actor, action, target, detail).await?;
    tx.commit()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "AUDIT_FAILED"))?;
    Ok(())
}

/// Acquire the single-instance PostgreSQL advisory lock.
///
/// The returned connection holds the session-level lock; callers keep it alive
/// for the process lifetime and drop it on shutdown (`docs/24` §6).
pub async fn acquire_instance_lock(
    pool: &sqlx::PgPool,
    key: i64,
) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>, sqlx::Error> {
    let mut conn = pool.acquire().await?;
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(key)
        .fetch_one(&mut *conn)
        .await?;
    if !acquired {
        return Err(sqlx::Error::Protocol(
            "another zk-server instance already holds the advisory lock".to_owned(),
        ));
    }
    Ok(conn)
}

/// Apply the ordered, immutable migration list and verify recorded checksums.
///
/// Startup refuses to run when a recorded migration changed/disappeared or the
/// list is not strictly ordered (`docs/24` §5: "启动检测 … checksum 变化/顺序漂移
/// → 拒启（只允许新增）"). Already-applied migrations are skipped.
pub async fn run_migrations(
    pool: &sqlx::PgPool,
    migrations: &[(&str, &str)],
) -> Result<(), sqlx::Error> {
    let mut seen = BTreeSet::new();
    let mut previous: Option<&str> = None;
    for (id, sql) in migrations {
        if !seen.insert(*id) {
            return Err(sqlx::Error::Protocol(format!(
                "duplicate migration id in build: {id}"
            )));
        }
        if let Some(prev) = previous {
            if *id <= prev {
                return Err(sqlx::Error::Protocol(format!(
                    "migration order drift: {id} is not after {prev}"
                )));
            }
        }
        previous = Some(id);
        let _ = sql;
    }
    // The ledger table itself is created by the first (core) migration.
    let ledger: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('schema_migrations')::text")
            .fetch_one(pool)
            .await?;
    let applied: Vec<(String, String)> = if ledger.is_some() {
        sqlx::query_as("SELECT id, checksum FROM schema_migrations")
            .fetch_all(pool)
            .await?
    } else {
        Vec::new()
    };
    let checksum_of = |sql: &str| {
        let mut hash = Sha256::new();
        hash.update(sql.as_bytes());
        format!("{:x}", hash.finalize())
    };
    for (id, recorded) in &applied {
        let Some((_, sql)) = migrations.iter().find(|(candidate, _)| candidate == id) else {
            return Err(sqlx::Error::Protocol(format!(
                "applied migration missing from build (history deleted): {id}"
            )));
        };
        let actual = checksum_of(sql);
        if &actual != recorded {
            return Err(sqlx::Error::Protocol(format!(
                "migration checksum changed for {id}: recorded {recorded} != actual {actual}; \
                 migrations are immutable, append a forward-only migration instead"
            )));
        }
    }
    for (id, sql) in migrations {
        if applied.iter().any(|(applied_id, _)| applied_id == id) {
            continue;
        }
        let checksum = checksum_of(sql);
        sqlx::raw_sql(sql).execute(pool).await?;
        sqlx::query("INSERT INTO schema_migrations (id, checksum) VALUES ($1, $2)")
            .bind(id)
            .bind(checksum)
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// Bootstrap the first super admin when the users table is empty.
pub async fn ensure_bootstrap_admin(
    state: &AppState,
    username: &str,
    password: &str,
) -> Result<bool, sqlx::Error> {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&state.pool)
        .await?;
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)] // test-only
mod tests {
    use super::canonical_json;

    #[test]
    fn canonical_json_sorts_nested_object_keys() {
        let left = serde_json::json!({"b": 1, "a": {"y": true, "x": "z"}});
        let right = serde_json::json!({"a": {"x": "z", "y": true}, "b": 1});
        assert_eq!(canonical_json(&left), r#"{"a":{"x":"z","y":true},"b":1}"#);
        assert_eq!(canonical_json(&left), canonical_json(&right));
        assert_eq!(
            canonical_json(&serde_json::json!([3, {"b": 2, "a": 1}])),
            r#"[3,{"a":1,"b":2}]"#
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)] // test-only
mod permission_tests {
    use super::{constant_time_eq, permissions_for_role};

    #[test]
    fn constant_time_eq_compares_content_and_length() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "abcd"));
        assert!(constant_time_eq("", ""));
    }

    #[test]
    fn permissions_cover_documented_roles() {
        let employee = permissions_for_role("employee");
        assert!(employee.contains(&"gateway:chat"));
        assert!(employee.contains(&"skills:publish"));
        let auditor = permissions_for_role("auditor");
        assert!(auditor.contains(&"admin:audit:read"));
        assert!(!auditor.contains(&"admin:users"));
        let super_admin = permissions_for_role("super_admin");
        assert!(super_admin.contains(&"admin:providers"));
        assert!(super_admin.contains(&"wallet:self"));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only
mod request_context_tests {
    use super::{
        attach_request_id, current_request_id, request_id_from_headers, upgrade_required_body,
        with_request_id,
    };
    use axum::body::Body;
    use axum::http::{HeaderMap, HeaderValue, StatusCode};
    use axum::response::Response;

    #[test]
    fn request_id_reads_or_generates_uuid() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", HeaderValue::from_static("req-123"));
        assert_eq!(request_id_from_headers(&headers), "req-123");
        let generated = request_id_from_headers(&HeaderMap::new());
        assert!(uuid::Uuid::parse_str(&generated).is_ok(), "{generated}");
    }

    #[test]
    fn upgrade_body_is_nested_without_fake_values() {
        let body = upgrade_required_body("req-9", Some("0.0.0"), "9.9.9", "too old");
        let error = &body["error"];
        assert_eq!(error["code"], "CLIENT_TOO_OLD");
        assert_eq!(error["retryable"], false);
        assert_eq!(error["request_id"], "req-9");
        assert_eq!(error["current"], "0.0.0");
        assert_eq!(error["minimum"], "9.9.9");
        assert_eq!(error["portal_url"], "/portal");
        assert_eq!(error["message"], "too old");
    }

    #[test]
    fn attach_request_id_sets_header() {
        let mut response = Response::new(Body::empty());
        attach_request_id(&mut response, "req-header");
        assert_eq!(
            response
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok()),
            Some("req-header")
        );
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn request_id_is_visible_inside_scope_only() {
        assert!(current_request_id().is_none());
        let value = with_request_id("req-scope".to_owned(), async { current_request_id() }).await;
        assert_eq!(value.as_deref(), Some("req-scope"));
        assert!(current_request_id().is_none());
    }
}
