#![forbid(unsafe_code)]
//! M2 P3/P4 system settings and client bootstrap.
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;

use zk_kit_server::AppState;
use zk_kit_server::{
    current_user as current_user_public, error as auth_error, require_admin,
    ServerError as AuthError,
};

#[derive(Debug, Deserialize)]
struct SettingInput {
    value: Value,
    #[serde(default)]
    hot: bool,
}

async fn admin_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AuthError> {
    require_admin(&state, &headers, false).await?;
    let rows = sqlx::query("SELECT key, value, hot, updated_at FROM system_settings ORDER BY key")
        .fetch_all(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(json!({
            "key": row.try_get::<String, _>("key").unwrap_or_default(),
            "value": row.try_get::<Value, _>("value").unwrap_or(Value::Null),
            "hot": row.try_get::<bool, _>("hot").unwrap_or(false),
        }));
    }
    Ok(Json(json!({ "ok": true, "items": items })))
}

async fn admin_put(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
    Json(request): Json<SettingInput>,
) -> Result<Json<Value>, AuthError> {
    let admin = require_admin(&state, &headers, true).await?;
    if key.trim().is_empty() {
        return Err(auth_error(StatusCode::BAD_REQUEST, "INVALID_KEY"));
    }
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query(
        "INSERT INTO system_settings (key, value, hot, updated_by) VALUES ($1, $2, $3, $4)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, hot = EXCLUDED.hot, updated_at = now(), updated_by = EXCLUDED.updated_by",
    )
    .bind(&key)
    .bind(&request.value)
    .bind(request.hot)
    .bind(admin.user_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    zk_kit_server::audit_tx(
        &mut tx,
        admin.user_id,
        "settings.update",
        &key,
        &json!({ "hot": request.hot, "value": request.value }),
    )
    .await?;
    tx.commit()
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(Json(json!({ "ok": true })))
}

async fn bootstrap(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AuthError> {
    let (user_id, role, _) = current_user_public(&state, &headers).await?;
    let models = sqlx::query("SELECT model, input_per_1k_micros, output_per_1k_micros FROM model_prices WHERE enabled = true ORDER BY model")
        .fetch_all(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let mut catalog = Vec::new();
    for row in models {
        catalog.push(json!({
            "id": row.try_get::<String, _>("model").unwrap_or_default(),
            "pricing": {
                "input_per_1k_micros": row.try_get::<i64, _>("input_per_1k_micros").unwrap_or(0),
                "output_per_1k_micros": row.try_get::<i64, _>("output_per_1k_micros").unwrap_or(0),
            },
        }));
    }
    let update_channel: Option<Value> =
        sqlx::query_scalar("SELECT value FROM system_settings WHERE key = 'updates.channel'")
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let payload = json!({
        "schema_version": 1,
        "identity": { "user_id": user_id, "role": role },
        "gateway": { "models": catalog, "default_model": catalog.first().and_then(|m| m.get("id")).cloned().unwrap_or(Value::Null) },
        "skills": [],
        "connectors": [],
        "apps": [],
        "updates": update_channel.unwrap_or(Value::Null),
        "brand": {},
        "config_etag": "",
    });
    Ok(Json(payload))
}

/// Admin settings + client bootstrap routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/settings", get(admin_list))
        .route("/api/admin/settings/{key}", put(admin_put))
        .route("/api/client/v1/config/bootstrap", get(bootstrap))
}

/// Immutable migrations owned by this domain.
pub const MIGRATIONS: &[(&str, &str)] = &[(
    "0007_settings",
    include_str!("../migrations/0007_settings.sql"),
)];
