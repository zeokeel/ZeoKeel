#![forbid(unsafe_code)]
//! Admin audit/usage read APIs (M2 P3).
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use sqlx::Row;

use zk_kit_server::AppState;
use zk_kit_server::{error as auth_error, require_admin, ServerError as AuthError};

async fn audit(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>, AuthError> {
    require_admin(&state, &headers, false).await?;
    let rows = sqlx::query("SELECT id, actor_id, action, target, detail, created_at FROM audit_log ORDER BY id DESC LIMIT 100")
        .fetch_all(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(json!({
            "id": row.try_get::<i64, _>("id").unwrap_or_default(),
            "actor_id": row.try_get::<Option<uuid::Uuid>, _>("actor_id").ok().flatten().map(|v| v.to_string()),
            "action": row.try_get::<String, _>("action").unwrap_or_default(),
            "target": row.try_get::<String, _>("target").unwrap_or_default(),
            "detail": row.try_get::<Value, _>("detail").unwrap_or(Value::Null),
        }));
    }
    Ok(Json(json!({ "ok": true, "items": items })))
}

async fn usage(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>, AuthError> {
    require_admin(&state, &headers, false).await?;
    let rows = sqlx::query(
        "SELECT u.username, count(*) AS requests, COALESCE(sum(g.input_tokens),0)::bigint AS input_tokens,
                COALESCE(sum(g.output_tokens),0)::bigint AS output_tokens, COALESCE(sum(g.amount_micros),0)::bigint AS cost_micros
         FROM usage g LEFT JOIN users u ON u.id = g.user_id
         GROUP BY u.username ORDER BY cost_micros DESC LIMIT 50",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(json!({
            "username": row.try_get::<Option<String>, _>("username").ok().flatten().unwrap_or_default(),
            "requests": row.try_get::<i64, _>("requests").unwrap_or_default(),
            "input_tokens": row.try_get::<i64, _>("input_tokens").unwrap_or_default(),
            "output_tokens": row.try_get::<i64, _>("output_tokens").unwrap_or_default(),
            "cost_micros": row.try_get::<i64, _>("cost_micros").unwrap_or_default(),
        }));
    }
    Ok(Json(json!({ "ok": true, "items": items })))
}

/// Audit/usage read routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/audit", get(audit))
        .route("/api/admin/usage", get(usage))
}

/// Immutable migrations owned by this domain.
pub const MIGRATIONS: &[(&str, &str)] = &[("0006_audit", include_str!("../migrations/0006_audit.sql"))];
