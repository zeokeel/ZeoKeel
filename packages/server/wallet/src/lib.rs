#![forbid(unsafe_code)]
//! M2 P2 wallet ledger slice: balance is derived only from `wallet_ledger`.
#![allow(clippy::too_many_lines)] // focused P2 slice

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

use zk_kit_server::AppState;
use zk_kit_server::{error as auth_error, require_admin, ServerError as AuthError};

#[derive(Debug, Deserialize)]
struct AdjustRequest {
    delta_micros: i64,
    reason: String,
    idempotency_key: String,
}

/// Create a wallet row if missing.
pub(crate) async fn ensure_wallet(state: &AppState, user_id: Uuid) -> Result<(), AuthError> {
    sqlx::query("INSERT INTO wallet_accounts (user_id, balance_micros) VALUES ($1, 0) ON CONFLICT (user_id) DO NOTHING")
        .bind(user_id)
        .execute(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(())
}

async fn mine(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>, AuthError> {
    let (user_id, _, _) = zk_kit_server::current_user(&state, &headers).await?;
    let uuid = Uuid::parse_str(&user_id).map_err(|_| auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    ensure_wallet(&state, uuid).await?;
    let balance: i64 = sqlx::query_scalar("SELECT balance_micros FROM wallet_accounts WHERE user_id = $1")
        .bind(uuid)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(Json(json!({
        "ok": true,
        "balance_micros": balance,
        "balance_display": format_micros(balance),
        "activated": true,
    })))
}

async fn ledger(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>, AuthError> {
    let (user_id, _, _) = zk_kit_server::current_user(&state, &headers).await?;
    let uuid = Uuid::parse_str(&user_id).map_err(|_| auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let rows = sqlx::query("SELECT delta_micros, balance_after, reason, ref, created_at FROM wallet_ledger WHERE user_id = $1 ORDER BY created_at DESC LIMIT 50")
        .bind(uuid)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(json!({
            "delta_micros": row.try_get::<i64, _>("delta_micros").unwrap_or_default(),
            "balance_after": row.try_get::<i64, _>("balance_after").unwrap_or_default(),
            "reason": row.try_get::<String, _>("reason").unwrap_or_default(),
            "ref": row.try_get::<String, _>("ref").unwrap_or_default(),
        }));
    }
    Ok(Json(json!({ "ok": true, "items": items })))
}

fn format_micros(micros: i64) -> String {
    let sign = if micros < 0 { "-" } else { "" };
    let absolute = micros.unsigned_abs();
    let whole = absolute / 1_000_000;
    let fraction = absolute % 1_000_000;
    format!("{sign}{whole}.{fraction:06}")
}

async fn admin_list(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>, AuthError> {
    require_admin(&state, &headers, false).await?;
    let rows = sqlx::query(
        "SELECT u.id, u.username, COALESCE(w.balance_micros, 0) AS balance_micros
         FROM users u LEFT JOIN wallet_accounts w ON w.user_id = u.id
         WHERE u.deleted_at IS NULL ORDER BY u.created_at ASC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(json!({
            "user_id": row.try_get::<Uuid, _>("id").map(|v| v.to_string()).unwrap_or_default(),
            "username": row.try_get::<String, _>("username").unwrap_or_default(),
            "balance_micros": row.try_get::<i64, _>("balance_micros").unwrap_or_default(),
        }));
    }
    Ok(Json(json!({ "ok": true, "items": items })))
}

async fn admin_adjust(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
    Json(request): Json<AdjustRequest>,
) -> Result<Json<Value>, AuthError> {
    let admin = require_admin(&state, &headers, true).await?;
    if request.delta_micros == 0 || request.idempotency_key.trim().is_empty() {
        return Err(auth_error(StatusCode::BAD_REQUEST, "INVALID_ADJUSTMENT"));
    }
    let uuid = Uuid::parse_str(&user_id).map_err(|_| auth_error(StatusCode::BAD_REQUEST, "INVALID_USER"))?;
    let existing = sqlx::query("SELECT balance_after FROM wallet_ledger WHERE idempotency_key = $1")
        .bind(&request.idempotency_key)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if let Some(row) = existing {
        let balance: i64 = row.try_get("balance_after").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
        return Ok(Json(json!({ "ok": true, "idempotent": true, "balance_micros": balance })));
    }
    ensure_wallet(&state, uuid).await?;
    let mut tx = state.pool.begin().await.map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let balance: i64 = sqlx::query_scalar("SELECT balance_micros FROM wallet_accounts WHERE user_id = $1 FOR UPDATE")
        .bind(uuid)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let next = balance.checked_add(request.delta_micros).ok_or_else(|| auth_error(StatusCode::BAD_REQUEST, "AMOUNT_OVERFLOW"))?;
    if next < 0 {
        return Err(auth_error(StatusCode::CONFLICT, "INSUFFICIENT_BALANCE"));
    }
    sqlx::query("UPDATE wallet_accounts SET balance_micros = $1, updated_at = now() WHERE user_id = $2")
        .bind(next)
        .bind(uuid)
        .execute(&mut *tx)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query("INSERT INTO wallet_ledger (user_id, delta_micros, balance_after, reason, ref, idempotency_key) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(uuid)
        .bind(request.delta_micros)
        .bind(next)
        .bind(&request.reason)
        .bind(format!("admin:{}", admin.user_id))
        .bind(&request.idempotency_key)
        .execute(&mut *tx)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    tx.commit().await.map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(Json(json!({ "ok": true, "balance_micros": next })))
}

/// Wallet routes (client + admin).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/client/v1/wallet", get(mine))
        .route("/api/client/v1/wallet/ledger", get(ledger))
        .route("/api/admin/wallet", get(admin_list))
        .route("/api/admin/wallet/{user_id}/adjust", post(admin_adjust))
}

/// Immutable migrations owned by this domain.
pub const MIGRATIONS: &[(&str, &str)] = &[("0003_wallet", include_str!("../migrations/0003_wallet.sql"))];
