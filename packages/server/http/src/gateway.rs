//! M2 P2 AI gateway: admission, upstream forwarding, usage/ledger settlement.
//!
//! Only OpenAI Responses (`POST /v1/responses`) is accepted. Upstream credentials
//! stay server-side; the client never receives a base URL or key.
#![allow(clippy::too_many_lines)] // focused slice

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use crate::AppState;
use crate::auth::{auth_error, current_user_public, AuthError};

async fn responses(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, AuthError> {
    let (user_id, _, _) = current_user_public(&state, &headers).await?;
    let Some(model) = body.get("model").and_then(Value::as_str).map(str::to_owned) else {
        return Err(auth_error(StatusCode::BAD_REQUEST, "MODEL_REQUIRED"));
    };
    let price = sqlx::query("SELECT input_per_1k_micros, output_per_1k_micros, peak_multiplier_bps, offpeak_multiplier_bps FROM model_prices WHERE model = $1 AND enabled = true")
        .bind(&model)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| auth_error(StatusCode::PAYMENT_REQUIRED, "MODEL_NOT_PRICED"))?;
    let input_price: i64 = price.try_get("input_per_1k_micros").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let output_price: i64 = price.try_get("output_per_1k_micros").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let balance: i64 = sqlx::query_scalar("SELECT COALESCE((SELECT balance_micros FROM wallet_accounts WHERE user_id = $1), 0)")
        .bind(Uuid::parse_str(&user_id).map_err(|_| auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if balance <= 0 {
        return Err(auth_error(StatusCode::PAYMENT_REQUIRED, "WALLET_EXHAUSTED"));
    }
    let base = std::env::var("ZEOKEEL_UPSTREAM_BASE_URL")
        .map_err(|_| auth_error(StatusCode::SERVICE_UNAVAILABLE, "UPSTREAM_UNAVAILABLE"))?;
    let key = std::env::var("ZEOKEEL_UPSTREAM_API_KEY")
        .map_err(|_| auth_error(StatusCode::SERVICE_UNAVAILABLE, "UPSTREAM_UNAVAILABLE"))?;
    if !base.starts_with("https://") && !base.starts_with("http://127.0.0.1") && !base.starts_with("http://localhost") {
        return Err(auth_error(StatusCode::SERVICE_UNAVAILABLE, "UPSTREAM_NOT_ALLOWED"));
    }
    let request_id = headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map_or_else(|| Uuid::new_v4().to_string(), str::to_owned);
    let client = reqwest::Client::new();
    let upstream = client
        .post(format!("{}/responses", base.trim_end_matches('/')))
        .bearer_auth(key)
        .json(&body)
        .send()
        .await
        .map_err(|_| auth_error(StatusCode::BAD_GATEWAY, "UPSTREAM"))?;
    let status = upstream.status();
    let content_type = upstream
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/json")
        .to_owned();
    let bytes = upstream.bytes().await.map_err(|_| auth_error(StatusCode::BAD_GATEWAY, "UPSTREAM"))?;
    let usage = extract_usage(&bytes, &content_type);
    let (input_tokens, output_tokens) = usage;
    if status.is_success() {
        let user_id = Uuid::parse_str(&user_id).map_err(|_| auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
        settle(&state, UsageSettlement { user_id, request_id: &request_id, model: &model, input_tokens, output_tokens, input_price, output_price }).await?;
    }
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    if let Ok(value) = HeaderValue::from_str(&content_type) {
        response.headers_mut().insert(header::CONTENT_TYPE, value);
    }
    Ok(response)
}

fn extract_usage(bytes: &[u8], content_type: &str) -> (i64, i64) {
    if content_type.contains("text/event-stream") {
        for line in String::from_utf8_lossy(bytes).lines() {
            let Some(data) = line.strip_prefix("data:") else { continue };
            let Ok(value) = serde_json::from_str::<Value>(data.trim()) else { continue };
            if let Some(usage) = value.get("response").and_then(|r| r.get("usage")).or_else(|| value.get("usage")) {
                return (usage.get("input_tokens").and_then(Value::as_i64).unwrap_or(0), usage.get("output_tokens").and_then(Value::as_i64).unwrap_or(0));
            }
        }
        return (0, 0);
    }
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else { return (0, 0) };
    let usage = value.get("usage");
    (usage.and_then(|u| u.get("input_tokens")).and_then(Value::as_i64).unwrap_or(0),
     usage.and_then(|u| u.get("output_tokens")).and_then(Value::as_i64).unwrap_or(0))
}

struct UsageSettlement<'a> {
    user_id: Uuid,
    request_id: &'a str,
    model: &'a str,
    input_tokens: i64,
    output_tokens: i64,
    input_price: i64,
    output_price: i64,
}

async fn settle(state: &AppState, settlement: UsageSettlement<'_>) -> Result<(), AuthError> {
    let UsageSettlement { user_id, request_id, model, input_tokens, output_tokens, input_price, output_price } = settlement;
    let input_cost = input_tokens.saturating_mul(input_price).saturating_div(1000);
    let output_cost = output_tokens.saturating_mul(output_price).saturating_div(1000);
    let cost = input_cost.saturating_add(output_cost);
    let mut tx = state.pool.begin().await.map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let existing: Option<Uuid> = sqlx::query_scalar("SELECT id FROM usage WHERE request_id = $1")
        .bind(request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if existing.is_some() {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO usage (request_id, user_id, model, input_tokens, output_tokens, amount_micros, status)
         VALUES ($1, $2, $3, $4, $5, $6, 'success')",
    )
    .bind(request_id)
    .bind(user_id)
    .bind(model)
    .bind(input_tokens)
    .bind(output_tokens)
    .bind(cost)
    .execute(&mut *tx)
    .await
    .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if cost > 0 {
        let balance: i64 = sqlx::query_scalar("SELECT balance_micros FROM wallet_accounts WHERE user_id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
        let next = balance.saturating_sub(cost);
        sqlx::query("UPDATE wallet_accounts SET balance_micros = $1, updated_at = now() WHERE user_id = $2")
            .bind(next)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
        sqlx::query("INSERT INTO wallet_ledger (user_id, delta_micros, balance_after, reason, ref, idempotency_key) VALUES ($1, $2, $3, 'gateway', $4, $5)")
            .bind(user_id)
            .bind(0_i64.saturating_sub(cost))
            .bind(next)
            .bind(request_id)
            .bind(format!("gw:{request_id}"))
            .execute(&mut *tx)
            .await
            .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    }
    tx.commit().await.map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(())
}

/// Gateway routes.
pub fn routes() -> Router<AppState> {
    Router::new().route("/v1/responses", post(responses))
}

