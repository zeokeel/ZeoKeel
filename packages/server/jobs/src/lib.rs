#![forbid(unsafe_code)]
//! M2 operational jobs: hold expiry, pending usage cleanup, reconciliation, monthly grant.
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

use zk_kit_server::AppState;
use zk_kit_server::{error as auth_error, require_admin, ServerError as AuthError};

/// Run one named job once and record a job_run.
pub async fn run_job(pool: &PgPool, job: &str, idempotency: &str) -> Result<Value, String> {
    let already: Option<String> = sqlx::query_scalar("SELECT status FROM job_runs WHERE idempotency_key = $1")
        .bind(idempotency)
        .fetch_optional(pool)
        .await
        .map_err(|error| error.to_string())?;
    if already.is_some() {
        return Ok(json!({ "job": job, "skipped": true }));
    }
    sqlx::query("INSERT INTO job_runs (job, idempotency_key, status) VALUES ($1, $2, 'running')")
        .bind(job)
        .bind(idempotency)
        .execute(pool)
        .await
        .map_err(|error| error.to_string())?;
    let result = match job {
        "hold-expire" => sqlx::query("UPDATE wallet_holds SET status = 'expired' WHERE status = 'held' AND expires_at < now()")
            .execute(pool)
            .await
            .map(|done| json!({ "expired": done.rows_affected() }))
            .map_err(|error| error.to_string()),
        "usage-pending" => sqlx::query("UPDATE usage SET status = 'voided' WHERE status = 'pending' AND created_at < now() - interval '1 hour'")
            .execute(pool)
            .await
            .map(|done| json!({ "voided": done.rows_affected() }))
            .map_err(|error| error.to_string()),
        "wallet-reconcile" => sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM wallet_accounts w WHERE w.balance_micros <> COALESCE((SELECT sum(l.delta_micros) FROM wallet_ledger l WHERE l.user_id = w.user_id), 0)",
        )
        .fetch_one(pool)
        .await
        .map(|mismatches| json!({ "mismatches": mismatches }))
        .map_err(|error| error.to_string()),
        "wallet-grant" => monthly_grant(pool).await.map_err(|error| error.to_string()),
        other => Err(format!("unknown job: {other}")),
    };
    match &result {
        Ok(detail) => {
            sqlx::query("UPDATE job_runs SET status = 'done', detail = $1, finished_at = now() WHERE idempotency_key = $2")
                .bind(detail)
                .bind(idempotency)
                .execute(pool)
                .await
                .map_err(|error| error.to_string())?;
        }
        Err(error) => {
            sqlx::query("UPDATE job_runs SET status = 'failed', detail = $1, finished_at = now() WHERE idempotency_key = $2")
                .bind(json!({ "error": error }))
                .bind(idempotency)
                .execute(pool)
                .await
                .map_err(|error| error.to_string())?;
        }
    }
    result
}

async fn monthly_grant(pool: &PgPool) -> Result<Value, sqlx::Error> {
    let amount = std::env::var("ZEOKEEL_MONTHLY_GRANT_MICROS").ok().and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
    if amount <= 0 {
        return Ok(json!({ "granted": 0, "note": "ZEOKEEL_MONTHLY_GRANT_MICROS unset" }));
    }
    let overwrite = std::env::var("ZEOKEEL_MONTHLY_GRANT_MODE").is_ok_and(|mode| mode == "overwrite");
    let month = chrono::Utc::now().format("%Y-%m").to_string();
    let mut tx = pool.begin().await?;
    let rows = sqlx::query(
        "SELECT w.user_id, w.balance_micros FROM wallet_accounts w JOIN users u ON u.id = w.user_id
         WHERE u.status = 'active' AND u.deleted_at IS NULL",
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut granted = 0_i64;
    for row in rows {
        let user_id: uuid::Uuid = row.try_get("user_id")?;
        let balance: i64 = row.try_get("balance_micros")?;
        let key = format!("grant:{user_id}:{month}");
        let exists: Option<uuid::Uuid> = sqlx::query_scalar("SELECT id FROM wallet_ledger WHERE idempotency_key = $1")
            .bind(&key)
            .fetch_optional(&mut *tx)
            .await?;
        if exists.is_some() {
            continue;
        }
        let delta = if overwrite { amount.saturating_sub(balance) } else { amount };
        let next = balance.saturating_add(delta);
        if next < 0 {
            continue;
        }
        sqlx::query("UPDATE wallet_accounts SET balance_micros = $1, updated_at = now() WHERE user_id = $2")
            .bind(next)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO wallet_ledger (user_id, delta_micros, balance_after, reason, ref, idempotency_key) VALUES ($1, $2, $3, 'monthly grant', $4, $5)")
            .bind(user_id)
            .bind(delta)
            .bind(next)
            .bind(&month)
            .bind(&key)
            .execute(&mut *tx)
            .await?;
        granted = granted.saturating_add(1);
    }
    tx.commit().await?;
    Ok(json!({ "granted": granted, "month": month, "mode": if overwrite { "overwrite" } else { "accumulate" } }))
}

async fn list(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>, AuthError> {
    require_admin(&state, &headers, false).await?;
    let rows = sqlx::query("SELECT job, idempotency_key, status, detail, started_at, finished_at FROM job_runs ORDER BY started_at DESC LIMIT 50")
        .fetch_all(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(json!({
            "job": row.try_get::<String, _>("job").unwrap_or_default(),
            "idempotency_key": row.try_get::<String, _>("idempotency_key").unwrap_or_default(),
            "status": row.try_get::<String, _>("status").unwrap_or_default(),
            "detail": row.try_get::<Value, _>("detail").unwrap_or(Value::Null),
        }));
    }
    Ok(Json(json!({ "ok": true, "items": items })))
}

async fn trigger(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(job): Path<String>,
) -> Result<Json<Value>, AuthError> {
    let _ = require_admin(&state, &headers, true).await?;
    let key = format!("{job}:{}", uuid::Uuid::new_v4());
    let detail = run_job(&state.pool, &job, &key)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "JOB_FAILED"))?;
    Ok(Json(json!({ "ok": true, "detail": detail })))
}

/// Start the background loop (5 min interval).
pub fn spawn_loop(pool: PgPool) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(300));
        loop {
            ticker.tick().await;
            for job in ["hold-expire", "usage-pending", "wallet-reconcile", "wallet-grant"] {
                let key = format!("{job}:{}", chrono::Utc::now().format("%Y%m%d%H"));
                if let Err(error) = run_job(&pool, job, &key).await {
                    eprintln!("job {job} failed: {error}");
                }
            }
        }
    });
}

/// Admin job routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/jobs", get(list))
        .route("/api/admin/jobs/{job}/run", post(trigger))
}
