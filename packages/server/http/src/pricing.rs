//! M2 P2 pricing and model catalog (admin editable, client read-only).
#![allow(clippy::too_many_lines)] // focused slice

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;

use crate::AppState;
use crate::auth::{auth_error, require_admin, AuthError};

#[derive(Debug, Deserialize)]
struct PriceInput {
    input_per_1k_micros: i64,
    output_per_1k_micros: i64,
    #[serde(default)]
    cached_per_1k_micros: i64,
    #[serde(default = "default_bps")]
    peak_multiplier_bps: i32,
    #[serde(default = "default_bps")]
    offpeak_multiplier_bps: i32,
    #[serde(default = "default_enabled")]
    enabled: bool,
}

fn default_bps() -> i32 {
    10_000
}

fn default_enabled() -> bool {
    true
}

async fn list(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>, AuthError> {
    require_admin(&state, &headers, false).await?;
    let rows = sqlx::query("SELECT model, input_per_1k_micros, output_per_1k_micros, cached_per_1k_micros, peak_multiplier_bps, offpeak_multiplier_bps, enabled FROM model_prices ORDER BY model")
        .fetch_all(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(json!({
            "model": row.try_get::<String, _>("model").unwrap_or_default(),
            "input_per_1k_micros": row.try_get::<i64, _>("input_per_1k_micros").unwrap_or_default(),
            "output_per_1k_micros": row.try_get::<i64, _>("output_per_1k_micros").unwrap_or_default(),
            "cached_per_1k_micros": row.try_get::<i64, _>("cached_per_1k_micros").unwrap_or_default(),
            "peak_multiplier_bps": row.try_get::<i32, _>("peak_multiplier_bps").unwrap_or(10_000),
            "offpeak_multiplier_bps": row.try_get::<i32, _>("offpeak_multiplier_bps").unwrap_or(10_000),
            "enabled": row.try_get::<bool, _>("enabled").unwrap_or(false),
        }));
    }
    Ok(Json(json!({ "ok": true, "items": items })))
}

async fn upsert(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(model): Path<String>,
    Json(request): Json<PriceInput>,
) -> Result<Json<Value>, AuthError> {
    let admin = require_admin(&state, &headers, true).await?;
    if model.trim().is_empty() || request.input_per_1k_micros < 0 || request.output_per_1k_micros < 0 {
        return Err(auth_error(StatusCode::BAD_REQUEST, "INVALID_PRICE"));
    }
    sqlx::query(
        "INSERT INTO model_prices (model, input_per_1k_micros, output_per_1k_micros, cached_per_1k_micros, peak_multiplier_bps, offpeak_multiplier_bps, enabled)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (model) DO UPDATE SET input_per_1k_micros = EXCLUDED.input_per_1k_micros,
           output_per_1k_micros = EXCLUDED.output_per_1k_micros, cached_per_1k_micros = EXCLUDED.cached_per_1k_micros,
           peak_multiplier_bps = EXCLUDED.peak_multiplier_bps, offpeak_multiplier_bps = EXCLUDED.offpeak_multiplier_bps,
           enabled = EXCLUDED.enabled, updated_at = now()",
    )
    .bind(&model)
    .bind(request.input_per_1k_micros)
    .bind(request.output_per_1k_micros)
    .bind(request.cached_per_1k_micros)
    .bind(request.peak_multiplier_bps)
    .bind(request.offpeak_multiplier_bps)
    .bind(request.enabled)
    .execute(&state.pool)
    .await
    .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    crate::users::audit(&state, admin.user_id, "pricing.upsert", &model, &json!({ "enabled": request.enabled })).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn models(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>, AuthError> {
    let _ = crate::auth::current_user_public(&state, &headers).await?;
    let rows = sqlx::query("SELECT model, input_per_1k_micros, output_per_1k_micros FROM model_prices WHERE enabled = true ORDER BY model")
        .fetch_all(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let mut data = Vec::new();
    for row in rows {
        let input: i64 = row.try_get("input_per_1k_micros").unwrap_or(0);
        let output: i64 = row.try_get("output_per_1k_micros").unwrap_or(0);
        data.push(json!({
            "id": row.try_get::<String, _>("model").unwrap_or_default(),
            "pricing": { "input_per_1k_micros": input, "output_per_1k_micros": output },
        }));
    }
    Ok(Json(json!({ "object": "list", "data": data })))
}

/// Pricing/admin + client model catalog routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/pricing", get(list))
        .route("/api/admin/pricing/{model}", put(upsert))
        .route("/v1/models", get(models))
}
