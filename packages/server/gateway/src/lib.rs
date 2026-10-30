#![forbid(unsafe_code)]
//! M2 P2 AI gateway: admission, upstream forwarding, usage/ledger settlement.
//!
//! Only OpenAI Responses (`POST /v1/responses`) is accepted. Upstream credentials
//! stay server-side; the client never receives a base URL or key.
//!
//! Billing invariants (`docs/22` §4–§5):
//! - admission (hold + `usage(pending)`) is one transaction with a wallet row lock;
//! - the settlement/idempotency key is server-generated per admission, never the
//!   client-supplied `x-request-id` header;
//! - streaming usage is read from the final `response.completed` event and the
//!   hold is settled (or voided) on every exit path, including disconnect;
//! - `settle` keeps `usage`/ledger/hold in one transaction and is retry-idempotent.
#![allow(clippy::too_many_lines)] // focused slice

use std::time::Duration;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

use zk_kit_server::AppState;
use zk_kit_server::{
    current_user as current_user_public, error as auth_error, ServerError as AuthError,
};

const DEFAULT_OUTPUT_TOKENS: i64 = 1024;
const DEFAULT_UPSTREAM_TIMEOUT_SECS: u64 = 120;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct UsageTotals {
    input_tokens: i64,
    cached_tokens: i64,
    output_tokens: i64,
    reasoning_tokens: i64,
}

async fn responses(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Response, AuthError> {
    let (user_id, _, _) = current_user_public(&state, &headers).await?;
    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|_| auth_error(StatusCode::UNAUTHORIZED, "AUTH_REQUIRED"))?;
    let Some(model) = body.get("model").and_then(Value::as_str).map(str::to_owned) else {
        return Err(auth_error(StatusCode::BAD_REQUEST, "MODEL_REQUIRED"));
    };
    let price = sqlx::query("SELECT input_per_1k_micros, output_per_1k_micros, cached_per_1k_micros, peak_multiplier_bps, offpeak_multiplier_bps FROM model_prices WHERE model = $1 AND enabled = true")
        .bind(&model)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| auth_error(StatusCode::PAYMENT_REQUIRED, "MODEL_NOT_PRICED"))?;
    let input_price: i64 = price
        .try_get("input_per_1k_micros")
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let output_price: i64 = price
        .try_get("output_per_1k_micros")
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let cached_price: i64 = price
        .try_get("cached_per_1k_micros")
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let peak_bps: i32 = price
        .try_get("peak_multiplier_bps")
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let offpeak_bps: i32 = price
        .try_get("offpeak_multiplier_bps")
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;

    // Validate upstream configuration before admission so a misconfigured server
    // never leaks a hold.
    let base = std::env::var("ZEOKEEL_UPSTREAM_BASE_URL")
        .map_err(|_| auth_error(StatusCode::SERVICE_UNAVAILABLE, "UPSTREAM_UNAVAILABLE"))?;
    let key = std::env::var("ZEOKEEL_UPSTREAM_API_KEY")
        .map_err(|_| auth_error(StatusCode::SERVICE_UNAVAILABLE, "UPSTREAM_UNAVAILABLE"))?;
    if !base.starts_with("https://")
        && !base.starts_with("http://127.0.0.1")
        && !base.starts_with("http://localhost")
    {
        return Err(auth_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "UPSTREAM_NOT_ALLOWED",
        ));
    }
    let upstream_timeout = std::env::var("ZEOKEEL_UPSTREAM_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_UPSTREAM_TIMEOUT_SECS);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(upstream_timeout))
        .build()
        .map_err(|_| auth_error(StatusCode::SERVICE_UNAVAILABLE, "UPSTREAM_UNAVAILABLE"))?;

    let max_output = body
        .get("max_output_tokens")
        .and_then(Value::as_i64)
        .unwrap_or(DEFAULT_OUTPUT_TOKENS)
        .max(1);
    let input_chars = estimate_input_chars(&body);
    let input_estimate = input_chars.saturating_div(4).saturating_add(1);
    let input_cost_estimate = amount_micros(input_estimate, input_price);
    let output_cost_estimate = amount_micros(max_output, output_price);
    let multiplier_bps = if is_peak_now() { peak_bps } else { offpeak_bps };
    let estimate = apply_multiplier(
        input_cost_estimate.saturating_add(output_cost_estimate),
        multiplier_bps,
    );

    // Server-generated settlement key: a client cannot force two distinct
    // upstream calls onto one idempotency key (P0: free re-use of x-request-id).
    let request_id = Uuid::new_v4().to_string();
    admit(
        &state,
        user_uuid,
        &request_id,
        &model,
        estimate,
        body.get("stream").and_then(Value::as_bool) == Some(true),
    )
    .await?;

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
        // accumulates usage and settles the ledger when the upstream ends. The task
        // keeps draining upstream after a client disconnect so the generated tokens
        // are still metered (`docs/22` §5.2: 断线按已生成 token 结算).
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<axum::body::Bytes, std::io::Error>>(32);
        let settlement = UsageSettlement {
            user_id: user_uuid,
            request_id: request_id.clone(),
            model: model.clone(),
            usage: UsageTotals::default(),
            input_price,
            output_price,
            cached_price,
            multiplier_bps,
            price_version: price_version(),
            allow_overdraft: true,
        };
        let state_for_task = state.clone();
        tokio::spawn(async move {
            use futures::StreamExt;
            let mut stream = upstream.bytes_stream();
            let mut text = String::new();
            let mut client_closed = false;
            let mut upstream_failed = false;
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        text.push_str(&String::from_utf8_lossy(&bytes));
                        if !client_closed && tx.send(Ok(bytes)).await.is_err() {
                            // Client went away; keep draining for usage.
                            client_closed = true;
                        }
                    }
                    Err(error) => {
                        if !client_closed {
                            let _ = tx.send(Err(std::io::Error::other(error))).await;
                        }
                        upstream_failed = true;
                        break;
                    }
                }
            }
            drop(tx);
            if upstream_failed {
                if let Err(error) = release_hold(&state_for_task, &settlement.request_id).await {
                    eprintln!(
                        "gateway: release hold {} after upstream failure: {:?}",
                        settlement.request_id, error.status
                    );
                }
                return;
            }
            let mut settlement = settlement;
            settlement.usage = extract_usage(text.as_bytes(), "text/event-stream");
            if let Err(error) = settle(&state_for_task, settlement).await {
                // Never drop settlement errors silently: void the hold and log so
                // operator reconciliation can find the request.
                eprintln!(
                    "gateway: settle failed for {}: {:?}",
                    request_id, error.status
                );
                if let Err(release_error) = release_hold(&state_for_task, &request_id).await {
                    eprintln!(
                        "gateway: release hold {} after settle failure: {:?}",
                        request_id, release_error.status
                    );
                }
            }
        });
        let body_stream = futures::stream::unfold(rx, |mut receiver| async move {
            receiver.recv().await.map(|item| (item, receiver))
        });
        let mut response = Response::new(Body::from_stream(body_stream));
        *response.status_mut() =
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
        if let Ok(value) = HeaderValue::from_str(&content_type) {
            response.headers_mut().insert(header::CONTENT_TYPE, value);
        }
        return Ok(response);
    }

    let bytes = upstream
        .bytes()
        .await
        .map_err(|_| auth_error(StatusCode::BAD_GATEWAY, "UPSTREAM"))?;
    if !status.is_success() {
        release_hold(&state, &request_id).await?;
    }
    if status.is_success() {
        settle(
            &state,
            UsageSettlement {
                user_id: user_uuid,
                request_id: request_id.clone(),
                model: model.clone(),
                usage: extract_usage(&bytes, &content_type),
                input_price,
                output_price,
                cached_price,
                multiplier_bps,
                price_version: price_version(),
                allow_overdraft: false,
            },
        )
        .await?;
    }
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() =
        StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    if let Ok(value) = HeaderValue::from_str(&content_type) {
        response.headers_mut().insert(header::CONTENT_TYPE, value);
    }
    Ok(response)
}

/// Insert the hold and the `usage(pending)` row in one transaction with a wallet
/// row lock, so concurrent requests cannot both pass the same headroom check
/// (`docs/22` §4).
async fn admit(
    state: &AppState,
    user_id: Uuid,
    request_id: &str,
    model: &str,
    estimate: i64,
    streaming: bool,
) -> Result<(), AuthError> {
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query("INSERT INTO wallet_accounts (user_id, balance_micros) VALUES ($1, 0) ON CONFLICT (user_id) DO NOTHING")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let balance: i64 = sqlx::query_scalar(
        "SELECT balance_micros FROM wallet_accounts WHERE user_id = $1 FOR UPDATE",
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if balance <= 0 && !streaming {
        return Err(auth_error(StatusCode::PAYMENT_REQUIRED, "WALLET_EXHAUSTED"));
    }
    let holds: i64 = sqlx::query_scalar("SELECT COALESCE(sum(amount_micros), 0)::bigint FROM wallet_holds WHERE user_id = $1 AND status = 'held' AND expires_at > now()")
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if balance.saturating_sub(holds) < estimate {
        return Err(auth_error(StatusCode::PAYMENT_REQUIRED, "WALLET_EXHAUSTED"));
    }
    sqlx::query("INSERT INTO wallet_holds (user_id, request_id, amount_micros, expires_at) VALUES ($1, $2, $3, now() + interval '5 minutes')")
        .bind(user_id)
        .bind(request_id)
        .bind(estimate)
        .execute(&mut *tx)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query("INSERT INTO usage_pending (user_id, request_id, model, estimated_micros) VALUES ($1, $2, $3, $4)")
        .bind(user_id)
        .bind(request_id)
        .bind(model)
        .bind(estimate)
        .execute(&mut *tx)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    tx.commit()
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(())
}

fn char_count(text: &str) -> i64 {
    i64::try_from(text.chars().count()).unwrap_or(i64::MAX)
}

fn value_char_count(value: &Value) -> i64 {
    match value {
        Value::String(text) => char_count(text),
        other => serde_json::to_string(other).map_or(0, |text| char_count(&text)),
    }
}

/// Conservative prompt-size estimate for the admission hold.
///
/// The Responses API accepts either a plain string `input` or an array of input
/// items (messages with `content` parts, `function_call_output`, function call
/// arguments, ...). The rig provider sends the array form, so a string-only
/// estimate would under-reserve the hold. `instructions` and `tools` also travel
/// in the prompt and are counted.
fn estimate_input_chars(body: &Value) -> i64 {
    let mut total = body
        .get("instructions")
        .map_or(0_i64, |value| value.as_str().map_or(0, char_count));
    if let Some(input) = body.get("input") {
        total = total.saturating_add(match input {
            Value::String(text) => char_count(text),
            Value::Array(items) => {
                let mut count = 0_i64;
                for item in items {
                    count = count.saturating_add(input_item_chars(item));
                }
                count
            }
            other => value_char_count(other),
        });
    }
    if let Some(tools) = body.get("tools") {
        total = total.saturating_add(value_char_count(tools));
    }
    total
}

fn input_item_chars(item: &Value) -> i64 {
    let Some(object) = item.as_object() else {
        return value_char_count(item);
    };
    let mut total = 0_i64;
    if let Some(content) = object.get("content") {
        total = total.saturating_add(match content {
            Value::String(text) => char_count(text),
            Value::Array(parts) => {
                let mut count = 0_i64;
                for part in parts {
                    count = count.saturating_add(part_chars(part));
                }
                count
            }
            other => value_char_count(other),
        });
    }
    for key in ["output", "arguments", "text"] {
        if let Some(value) = object.get(key) {
            total = total.saturating_add(value_char_count(value));
        }
    }
    if total == 0 {
        return value_char_count(item);
    }
    total
}

fn part_chars(part: &Value) -> i64 {
    part.get("text")
        .and_then(Value::as_str)
        .map_or_else(|| value_char_count(part), char_count)
}

fn price_version() -> String {
    std::env::var("ZEOKEEL_PRICE_VERSION").unwrap_or_default()
}

fn amount_micros(tokens: i64, price_per_1k: i64) -> i64 {
    tokens
        .saturating_mul(price_per_1k)
        .saturating_add(500)
        .saturating_div(1000)
}

fn apply_multiplier(amount: i64, multiplier_bps: i32) -> i64 {
    amount
        .saturating_mul(i64::from(multiplier_bps))
        .saturating_add(5000)
        .saturating_div(10_000)
}

fn billed_amount(
    usage: UsageTotals,
    input_price: i64,
    cached_price: i64,
    output_price: i64,
) -> i64 {
    let cached = usage.cached_tokens.max(0);
    let billable_input = usage.input_tokens.max(0).saturating_sub(cached);
    amount_micros(billable_input, input_price)
        .saturating_add(amount_micros(cached, cached_price))
        .saturating_add(amount_micros(usage.output_tokens.max(0), output_price))
}

/// Extract token usage from a Responses payload.
///
/// Real Responses SSE streams start with `response.created` carrying
/// `usage: null`; the authoritative counters arrive in `response.completed`
/// (or a later `response.*` event). We therefore skip null/empty usage objects
/// and keep scanning, preferring the last non-empty usage seen.
fn extract_usage(bytes: &[u8], content_type: &str) -> UsageTotals {
    if content_type.contains("text/event-stream") {
        let mut totals = UsageTotals::default();
        for line in String::from_utf8_lossy(bytes).lines() {
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let Ok(value) = serde_json::from_str::<Value>(data.trim()) else {
                continue;
            };
            if let Some(usage) = usage_from_value(&value) {
                totals = usage;
            }
        }
        return totals;
    }
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return UsageTotals::default();
    };
    usage_from_value(&value).unwrap_or_default()
}

fn usage_from_value(value: &Value) -> Option<UsageTotals> {
    let usage = value.get("usage").or_else(|| {
        value
            .get("response")
            .and_then(|response| response.get("usage"))
    })?;
    if usage.is_null() || !usage.is_object() {
        return None;
    }
    let has_tokens = usage.get("input_tokens").is_some() || usage.get("output_tokens").is_some();
    if !has_tokens {
        return None;
    }
    let cached_tokens = usage
        .get("input_tokens_details")
        .and_then(|details| details.get("cached_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let reasoning_tokens = usage
        .get("output_tokens_details")
        .and_then(|details| details.get("reasoning_tokens"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    Some(UsageTotals {
        input_tokens: usage
            .get("input_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        cached_tokens,
        output_tokens: usage
            .get("output_tokens")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        reasoning_tokens,
    })
}

fn is_peak_now() -> bool {
    use chrono::Timelike;
    let hour = chrono::Local::now().hour();
    (9..18).contains(&hour)
}

/// Configured lower bound for streaming overdraft (micros below zero).
///
/// `docs/22` §4 defaults the floor to one monthly grant allocation; an explicit
/// `ZEOKEEL_STREAM_NEGATIVE_FLOOR_MICROS` overrides it.
fn stream_negative_floor() -> i64 {
    if let Some(value) = std::env::var("ZEOKEEL_STREAM_NEGATIVE_FLOOR_MICROS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
    {
        return value.max(0);
    }
    std::env::var("ZEOKEEL_MONTHLY_GRANT_MICROS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0)
        .max(0)
}

async fn release_hold(state: &AppState, request_id: &str) -> Result<(), AuthError> {
    sqlx::query(
        "UPDATE wallet_holds SET status = 'void' WHERE request_id = $1 AND status = 'held'",
    )
    .bind(request_id)
    .execute(&state.pool)
    .await
    .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query("UPDATE usage_pending SET status = 'void', settled_at = now() WHERE request_id = $1 AND status = 'pending'")
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
    usage: UsageTotals,
    input_price: i64,
    output_price: i64,
    cached_price: i64,
    multiplier_bps: i32,
    price_version: String,
    allow_overdraft: bool,
}

async fn settle(state: &AppState, settlement: UsageSettlement) -> Result<(), AuthError> {
    let UsageSettlement {
        user_id,
        request_id,
        model,
        usage,
        input_price,
        output_price,
        cached_price,
        multiplier_bps,
        price_version,
        allow_overdraft,
    } = settlement;
    let base = billed_amount(usage, input_price, cached_price, output_price);
    let cost = apply_multiplier(base, multiplier_bps);
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    // Retry-idempotent: a retried settlement closes the hold/pending rows and
    // returns without a second charge.
    let existing: Option<Uuid> = sqlx::query_scalar("SELECT id FROM usage WHERE request_id = $1")
        .bind(&request_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if existing.is_some() {
        mark_settled(&mut tx, &request_id).await?;
        tx.commit()
            .await
            .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
        return Ok(());
    }
    let balance: i64 = sqlx::query_scalar(
        "SELECT balance_micros FROM wallet_accounts WHERE user_id = $1 FOR UPDATE",
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
    .unwrap_or(0);
    let floor = if allow_overdraft {
        0_i64.saturating_sub(stream_negative_floor())
    } else {
        0
    };
    let next = balance.saturating_sub(cost).max(floor);
    let charged = balance.saturating_sub(next);
    let status = if charged < cost {
        "aborted"
    } else if usage.input_tokens == 0 && usage.output_tokens == 0 {
        "estimated"
    } else {
        "success"
    };
    sqlx::query(
        "INSERT INTO usage (request_id, user_id, model, input_tokens, cached_tokens, output_tokens, reasoning_tokens, amount_micros, status, price_version, multiplier_bps, settled_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, now())",
    )
    .bind(&request_id)
    .bind(user_id)
    .bind(&model)
    .bind(usage.input_tokens)
    .bind(usage.cached_tokens)
    .bind(usage.output_tokens)
    .bind(usage.reasoning_tokens)
    .bind(charged)
    .bind(status)
    .bind(&price_version)
    .bind(multiplier_bps)
    .execute(&mut *tx)
    .await
    .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if charged > 0 {
        sqlx::query(
            "UPDATE wallet_accounts SET balance_micros = $1, updated_at = now() WHERE user_id = $2",
        )
        .bind(next)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
        sqlx::query("INSERT INTO wallet_ledger (user_id, delta_micros, balance_after, reason, ref, idempotency_key) VALUES ($1, $2, $3, 'gateway', $4, $5)")
            .bind(user_id)
            .bind(0_i64.saturating_sub(charged))
            .bind(next)
            .bind(&request_id)
            .bind(format!("gw:{request_id}"))
            .execute(&mut *tx)
            .await
            .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    }
    mark_settled(&mut tx, &request_id).await?;
    tx.commit()
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(())
}

async fn mark_settled(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request_id: &str,
) -> Result<(), AuthError> {
    sqlx::query(
        "UPDATE wallet_holds SET status = 'settled' WHERE request_id = $1 AND status = 'held'",
    )
    .bind(request_id)
    .execute(&mut **tx)
    .await
    .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query("UPDATE usage_pending SET status = 'settled', settled_at = now() WHERE request_id = $1 AND status = 'pending'")
        .bind(request_id)
        .execute(&mut **tx)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(())
}

/// Gateway routes.
pub fn routes() -> Router<AppState> {
    Router::new().route("/v1/responses", post(responses))
}

/// Immutable migrations owned by this domain.
pub const MIGRATIONS: &[(&str, &str)] = &[(
    "0005_gateway",
    include_str!("../migrations/0005_gateway.sql"),
)];

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)] // test-only
mod tests {
    use super::{extract_usage, UsageTotals};

    #[test]
    fn sse_real_shape_reads_response_completed_usage() {
        // Reproduces the P0: response.created carries usage:null and must not
        // short-circuit parsing of the final response.completed event.
        let body = "event: response.created\n\
                    data: {\"type\":\"response.created\",\"response\":{\"id\":\"r1\",\"usage\":null}}\n\n\
                    event: response.output_text.delta\n\
                    data: {\"type\":\"response.output_text.delta\",\"delta\":\"hi\"}\n\n\
                    event: response.completed\n\
                    data: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"input_tokens\":100,\"output_tokens\":100}}}\n\n";
        assert_eq!(
            extract_usage(body.as_bytes(), "text/event-stream"),
            UsageTotals {
                input_tokens: 100,
                cached_tokens: 0,
                output_tokens: 100,
                reasoning_tokens: 0,
            }
        );
    }

    #[test]
    fn sse_reads_cached_and_reasoning_details() {
        let body = "data: {\"response\":{\"usage\":null}}\n\n\
                    data: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"input_tokens\":40,\"input_tokens_details\":{\"cached_tokens\":10},\"output_tokens\":7,\"output_tokens_details\":{\"reasoning_tokens\":3}}}}\n\n";
        assert_eq!(
            extract_usage(body.as_bytes(), "text/event-stream"),
            UsageTotals {
                input_tokens: 40,
                cached_tokens: 10,
                output_tokens: 7,
                reasoning_tokens: 3,
            }
        );
    }

    #[test]
    fn sse_without_usage_is_zero() {
        let body = "data: {\"type\":\"response.created\",\"response\":{\"usage\":null}}\n\ndata: [DONE]\n\n";
        assert_eq!(
            extract_usage(body.as_bytes(), "text/event-stream"),
            UsageTotals::default()
        );
    }

    #[test]
    fn estimate_counts_string_input_and_instructions() {
        let body = serde_json::json!({"input": "12345678", "instructions": "abcd"});
        assert_eq!(super::estimate_input_chars(&body), 12);
    }

    #[test]
    fn estimate_counts_responses_input_array_and_tool_outputs() {
        // rig sends the array form; text parts and tool outputs must be reserved.
        let body = serde_json::json!({
            "input": [
                {"role": "user", "content": [{"type": "input_text", "text": "1234"}]},
                {"type": "function_call_output", "call_id": "c1", "output": "5678"},
                {"type": "message", "content": "90"}
            ]
        });
        assert_eq!(super::estimate_input_chars(&body), 10);
    }

    /// Real-PG regression for the P0-2 "retry of one admission is charged once"
    /// requirement: `settle` is retry-idempotent on the server-generated request
    /// id. Run via `scripts/srv-auth-test.sh` with `ZEOKEEL_TEST_DATABASE_URL`
    /// pointing at an already-migrated scratch database.
    #[tokio::test]
    #[ignore = "requires ZEOKEEL_TEST_DATABASE_URL (real PostgreSQL)"]
    async fn pg_settle_retry_charges_once() {
        use super::{settle, UsageSettlement, UsageTotals};
        use zk_kit_server::AppState;

        let Ok(url) = std::env::var("ZEOKEEL_TEST_DATABASE_URL") else {
            eprintln!("skipped: ZEOKEEL_TEST_DATABASE_URL unset");
            return;
        };
        let pool = sqlx::PgPool::connect(&url).await.expect("connect test db");
        let state = AppState::new(pool);
        let username = format!("settle-retry-{}", uuid::Uuid::new_v4());
        let user_id: uuid::Uuid = sqlx::query_scalar(
            "INSERT INTO users (username, role) VALUES ($1, 'employee') RETURNING id",
        )
        .bind(&username)
        .fetch_one(&state.pool)
        .await
        .expect("insert user");
        sqlx::query("INSERT INTO wallet_accounts (user_id, balance_micros) VALUES ($1, 1000000)")
            .bind(user_id)
            .execute(&state.pool)
            .await
            .expect("wallet");
        let request_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO wallet_holds (user_id, request_id, amount_micros, expires_at) VALUES ($1, $2, 1000, now() + interval '5 minutes')")
            .bind(user_id)
            .bind(&request_id)
            .execute(&state.pool)
            .await
            .expect("hold");
        sqlx::query("INSERT INTO usage_pending (user_id, request_id, model, estimated_micros) VALUES ($1, $2, 'deepseek-chat', 1000)")
            .bind(user_id)
            .bind(&request_id)
            .execute(&state.pool)
            .await
            .expect("pending usage");
        let settlement = || UsageSettlement {
            user_id,
            request_id: request_id.clone(),
            model: "deepseek-chat".to_owned(),
            usage: UsageTotals {
                input_tokens: 100,
                cached_tokens: 0,
                output_tokens: 100,
                reasoning_tokens: 0,
            },
            input_price: 1000,
            output_price: 2000,
            cached_price: 0,
            multiplier_bps: 10_000,
            price_version: String::new(),
            allow_overdraft: false,
        };
        settle(&state, settlement()).await.expect("first settle");
        settle(&state, settlement()).await.expect("retry settle");
        let usage_rows: i64 =
            sqlx::query_scalar("SELECT count(*) FROM usage WHERE request_id = $1")
                .bind(&request_id)
                .fetch_one(&state.pool)
                .await
                .expect("usage count");
        assert_eq!(usage_rows, 1, "retry must not insert a second usage row");
        let ledger_rows: i64 =
            sqlx::query_scalar("SELECT count(*) FROM wallet_ledger WHERE idempotency_key = $1")
                .bind(format!("gw:{request_id}"))
                .fetch_one(&state.pool)
                .await
                .expect("ledger count");
        assert_eq!(ledger_rows, 1, "retry must not debit twice");
        let balance: i64 =
            sqlx::query_scalar("SELECT balance_micros FROM wallet_accounts WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&state.pool)
                .await
                .expect("balance");
        assert_eq!(balance, 999_700, "charged exactly once (100+100 tokens)");
        let hold: String =
            sqlx::query_scalar("SELECT status FROM wallet_holds WHERE request_id = $1")
                .bind(&request_id)
                .fetch_one(&state.pool)
                .await
                .expect("hold status");
        assert_eq!(hold, "settled");
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&state.pool)
            .await
            .expect("cleanup user");
    }

    #[test]
    fn non_stream_usage_is_read() {
        let body = b"{\"id\":\"resp_1\",\"usage\":{\"input_tokens\":10,\"output_tokens\":20}}";
        assert_eq!(
            extract_usage(body, "application/json"),
            UsageTotals {
                input_tokens: 10,
                cached_tokens: 0,
                output_tokens: 20,
                reasoning_tokens: 0,
            }
        );
    }
}
