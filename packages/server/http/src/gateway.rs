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
    let peak_bps: i32 = price.try_get("peak_multiplier_bps").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let offpeak_bps: i32 = price.try_get("offpeak_multiplier_bps").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let balance: i64 = sqlx::query_scalar("SELECT COALESCE((SELECT balance_micros FROM wallet_accounts WHERE user_id = $1), 0)")
        .bind(Uuid::parse_str(&user_id).map_err(|_| auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if balance <= 0 {
        return Err(auth_error(StatusCode::PAYMENT_REQUIRED, "WALLET_EXHAUSTED"));
    }
    let user_uuid = Uuid::parse_str(&user_id).map_err(|_| auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let request_id = headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map_or_else(|| Uuid::new_v4().to_string(), str::to_owned);
    let max_output = body.get("max_output_tokens").and_then(Value::as_i64).unwrap_or(1024).max(1);
    let input_chars = body.get("input").map_or(0_i64, |value| value.as_str().map_or(0, |text| i64::try_from(text.chars().count()).unwrap_or(0)));
    let input_estimate = input_chars.saturating_div(4).saturating_add(1);
    let input_cost_estimate = input_estimate.saturating_mul(input_price).saturating_div(1000);
    let output_cost_estimate = max_output.saturating_mul(output_price).saturating_div(1000);
    let multiplier = if is_peak_now() { peak_bps } else { offpeak_bps };
    let estimate = input_cost_estimate.saturating_add(output_cost_estimate).saturating_mul(i64::from(multiplier)).saturating_div(10_000);
    let holds: i64 = sqlx::query_scalar("SELECT COALESCE(sum(amount_micros), 0)::bigint FROM wallet_holds WHERE user_id = $1 AND status = 'held' AND expires_at > now()")
        .bind(user_uuid)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if balance.saturating_sub(holds) < estimate {
        return Err(auth_error(StatusCode::PAYMENT_REQUIRED, "WALLET_EXHAUSTED"));
    }
    sqlx::query("INSERT INTO wallet_holds (user_id, request_id, amount_micros, expires_at) VALUES ($1, $2, $3, now() + interval '5 minutes')")
        .bind(user_uuid)
        .bind(&request_id)
        .bind(estimate)
        .execute(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let base = std::env::var("ZEOKEEL_UPSTREAM_BASE_URL")
        .map_err(|_| auth_error(StatusCode::SERVICE_UNAVAILABLE, "UPSTREAM_UNAVAILABLE"))?;
    let key = std::env::var("ZEOKEEL_UPSTREAM_API_KEY")
        .map_err(|_| auth_error(StatusCode::SERVICE_UNAVAILABLE, "UPSTREAM_UNAVAILABLE"))?;
    if !base.starts_with("https://") && !base.starts_with("http://127.0.0.1") && !base.starts_with("http://localhost") {
        return Err(auth_error(StatusCode::SERVICE_UNAVAILABLE, "UPSTREAM_NOT_ALLOWED"));
    }
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
    let wants_stream = body.get("stream").and_then(Value::as_bool) == Some(true);
    if wants_stream && status.is_success() {
        // Real SSE passthrough: stream chunks to the client while a background task
        // accumulates usage and settles the ledger when the upstream ends.
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<axum::body::Bytes, std::io::Error>>(32);
        let settlement = UsageSettlement { user_id: user_uuid, request_id: request_id.clone(), model: model.clone(), input_tokens: 0, output_tokens: 0, input_price, output_price, multiplier_bps: multiplier };
        let state_for_task = state.clone();
        tokio::spawn(async move {
            use futures::StreamExt;
            let mut stream = upstream.bytes_stream();
            let mut text = String::new();
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        text.push_str(&String::from_utf8_lossy(&bytes));
                        if tx.send(Ok(bytes)).await.is_err() {
                            return;
                        }
                    }
                    Err(error) => {
                        let _ = tx.send(Err(std::io::Error::other(error))).await;
                        return;
                    }
                }
            }
            let (input_tokens, output_tokens) = extract_usage(text.as_bytes(), "text/event-stream");
            let mut settlement = settlement;
            settlement.input_tokens = input_tokens;
            settlement.output_tokens = output_tokens;
            let _ = settle(&state_for_task, settlement).await;
        });
        let body_stream = futures::stream::unfold(rx, |mut receiver| async move {
            receiver.recv().await.map(|item| (item, receiver))
        });
        let mut response = Response::new(Body::from_stream(body_stream));
        *response.status_mut() = StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
        if let Ok(value) = HeaderValue::from_str(&content_type) {
            response.headers_mut().insert(header::CONTENT_TYPE, value);
        }
        return Ok(response);
    }
    let bytes = upstream.bytes().await.map_err(|_| auth_error(StatusCode::BAD_GATEWAY, "UPSTREAM"))?;
    if !status.is_success() {
        release_hold(&state, &request_id).await?;
    }
    let usage = extract_usage(&bytes, &content_type);
    let (input_tokens, output_tokens) = usage;
    if status.is_success() {
        settle(&state, UsageSettlement { user_id: user_uuid, request_id: request_id.clone(), model: model.clone(), input_tokens, output_tokens, input_price, output_price, multiplier_bps: multiplier }).await?;
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

fn is_peak_now() -> bool {
    use chrono::Timelike;
    let hour = chrono::Local::now().hour();
    (9..18).contains(&hour)
}

async fn release_hold(state: &AppState, request_id: &str) -> Result<(), AuthError> {
    sqlx::query("UPDATE wallet_holds SET status = 'void' WHERE request_id = $1 AND status = 'held'")
        .bind(request_id)
        .execute(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(())
}

struct UsageSettlement {
    user_id: Uuid,
    request_id: String,
    model: String,
    input_tokens: i64,
    output_tokens: i64,
    input_price: i64,
    output_price: i64,
    multiplier_bps: i32,
}

async fn settle(state: &AppState, settlement: UsageSettlement) -> Result<(), AuthError> {
    let UsageSettlement { user_id, request_id, model, input_tokens, output_tokens, input_price, output_price, multiplier_bps } = settlement;
    let input_cost = input_tokens.saturating_mul(input_price).saturating_div(1000);
    let output_cost = output_tokens.saturating_mul(output_price).saturating_div(1000);
    let base = input_cost.saturating_add(output_cost);
    let cost = base.saturating_mul(i64::from(multiplier_bps)).saturating_div(10_000);
    let mut tx = state.pool.begin().await.map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let existing: Option<Uuid> = sqlx::query_scalar("SELECT id FROM usage WHERE request_id = $1")
        .bind(&request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if existing.is_some() {
        return Ok(());
    }
    sqlx::query("UPDATE wallet_holds SET status = 'settled' WHERE request_id = $1 AND status = 'held'")
        .bind(&request_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query(
        "INSERT INTO usage (request_id, user_id, model, input_tokens, output_tokens, amount_micros, status)
         VALUES ($1, $2, $3, $4, $5, $6, 'success')",
    )
    .bind(&request_id)
    .bind(user_id)
    .bind(&model)
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
            .bind(&request_id)
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

