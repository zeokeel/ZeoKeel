//! MCP connector aggregate (`srv.connectors`): no static secrets (ADR-060).
#![forbid(unsafe_code)]

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;
use zk_kit_server::{audit, current_user, error, require_admin, AppState, ServerError};

/// Immutable migrations owned by this package.
pub const MIGRATIONS: &[(&str, &str)] = &[("0011_connectors", include_str!("../migrations/0011_connectors.sql"))];

/// Connector routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/connectors", get(admin_list).post(create))
        .route("/api/admin/connectors/{id}", axum::routing::delete(delete_connector))
        .route("/api/admin/connectors/{id}/grants", put(set_grants))
        .route("/api/admin/connectors/{id}/oauth", post(set_oauth))
        .route("/api/admin/connectors/{id}/force-update", post(force_update))
        .route("/api/client/v1/connectors", get(catalog))
        .route("/api/client/v1/connectors/{id}/{action}", post(toggle))
}

#[derive(Debug, Deserialize)]
struct CreateConnector {
    name: String,
    transport: String,
    #[serde(default)]
    config: Value,
}

#[derive(Debug, Deserialize)]
struct Grants { grants: Vec<zk_contract_connector::ConnectorGrant> }

#[derive(Debug, Deserialize)]
struct OAuthInput { client_id: String, #[serde(default)] scopes: Vec<String>, redirect_uri: String }

#[derive(Debug, Deserialize)]
struct ForceInput { #[serde(default = "default_true")] force: bool, #[serde(default)] min_client_version: String }

fn default_true() -> bool { true }

fn uuid_of(value: &str) -> Result<Uuid, ServerError> {
    Uuid::parse_str(value).map_err(|_| error(StatusCode::BAD_REQUEST, "VALIDATION"))
}

fn has_sensitive(config: &Value) -> bool {
    match config {
        Value::Object(map) => map.iter().any(|(key, value)| {
            let key = key.to_ascii_lowercase();
            matches!(key.as_str(), "secret" | "token" | "password" | "api_key" | "apikey" | "authorization" | "client_secret") || has_sensitive(value)
        }),
        Value::Array(items) => items.iter().any(has_sensitive),
        _ => false,
    }
}

fn transport_ok(value: &str) -> bool { matches!(value, "stdio" | "http" | "sse") }

async fn admin_list(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>, ServerError> {
    require_admin(&state, &headers, false).await?;
    let rows = sqlx::query(
        "SELECT c.id, c.name, c.transport, c.force_update, c.min_client_version,
                COALESCE((SELECT jsonb_agg(jsonb_build_object('grantee', g.grantee, 'can_enable', g.can_enable)) FROM connector_grants g WHERE g.connector_id = c.id), '[]'::jsonb) AS grants,
                o.client_id, o.redirect_uri
         FROM connectors c LEFT JOIN connector_oauth o ON o.connector_id = c.id
         WHERE c.deleted_at IS NULL ORDER BY c.name",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let items: Vec<Value> = rows.into_iter().map(|row| json!({
        "id": row.try_get::<Uuid, _>("id").map(|v| v.to_string()).unwrap_or_default(),
        "name": row.try_get::<String, _>("name").unwrap_or_default(),
        "transport": row.try_get::<String, _>("transport").unwrap_or_default(),
        "force_update": row.try_get::<bool, _>("force_update").unwrap_or(false),
        "min_client_version": row.try_get::<String, _>("min_client_version").unwrap_or_default(),
        "grants": row.try_get::<Value, _>("grants").unwrap_or(Value::Array(Vec::new())),
        "oauth_client_id": row.try_get::<Option<String>, _>("client_id").ok().flatten(),
        "redirect_uri": row.try_get::<Option<String>, _>("redirect_uri").ok().flatten(),
    })).collect();
    Ok(Json(json!({ "ok": true, "items": items })))
}

async fn create(State(state): State<AppState>, headers: HeaderMap, Json(body): Json<CreateConnector>) -> Result<Json<Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    if body.name.trim().is_empty() || !transport_ok(&body.transport) || has_sensitive(&body.config) {
        return Err(error(StatusCode::BAD_REQUEST, "VALIDATION"));
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO connectors (name, transport, config) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(body.name.trim())
    .bind(&body.transport)
    .bind(&body.config)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| error(StatusCode::CONFLICT, "CONFLICT"))?;
    audit(&state, admin.user_id, "connector.create", &id.to_string(), &json!({ "name": body.name, "transport": body.transport })).await?;
    Ok(Json(json!({ "ok": true, "id": id.to_string() })))
}

async fn delete_connector(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> Result<Json<Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    let id = uuid_of(&id)?;
    let result = sqlx::query("UPDATE connectors SET deleted_at = now(), updated_at = now() WHERE id = $1 AND deleted_at IS NULL")
        .bind(id).execute(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if result.rows_affected() == 0 { return Err(error(StatusCode::NOT_FOUND, "CONNECTOR_NOT_FOUND")); }
    audit(&state, admin.user_id, "connector.delete", &id.to_string(), &json!({})).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn set_grants(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>, Json(body): Json<Grants>) -> Result<Json<Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    let id = uuid_of(&id)?;
    if body.grants.is_empty() { return Err(error(StatusCode::BAD_REQUEST, "VALIDATION")); }
    let mut tx = state.pool.begin().await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query("DELETE FROM connector_grants WHERE connector_id = $1").bind(id).execute(&mut *tx).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    for grant in &body.grants {
        sqlx::query("INSERT INTO connector_grants (connector_id, grantee, can_enable) VALUES ($1, $2, $3)")
            .bind(id).bind(&grant.grantee).bind(grant.can_enable).execute(&mut *tx).await
            .map_err(|_| error(StatusCode::BAD_REQUEST, "VALIDATION"))?;
    }
    tx.commit().await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    audit(&state, admin.user_id, "connector.grants.update", &id.to_string(), &json!({})).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn set_oauth(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>, Json(body): Json<OAuthInput>) -> Result<Json<Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    let id = uuid_of(&id)?;
    if body.client_id.trim().is_empty() || body.redirect_uri.trim().is_empty() { return Err(error(StatusCode::BAD_REQUEST, "VALIDATION")); }
    sqlx::query(
        "INSERT INTO connector_oauth (connector_id, client_id, scopes, redirect_uri) VALUES ($1, $2, $3, $4)
         ON CONFLICT (connector_id) DO UPDATE SET client_id = EXCLUDED.client_id, scopes = EXCLUDED.scopes, redirect_uri = EXCLUDED.redirect_uri",
    )
    .bind(id).bind(body.client_id.trim()).bind(json!(body.scopes)).bind(body.redirect_uri.trim())
    .execute(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    audit(&state, admin.user_id, "connector.oauth.update", &id.to_string(), &json!({})).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn force_update(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>, Json(body): Json<ForceInput>) -> Result<Json<Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    let id = uuid_of(&id)?;
    sqlx::query("UPDATE connectors SET force_update = $1, min_client_version = $2, updated_at = now() WHERE id = $3 AND deleted_at IS NULL")
        .bind(body.force).bind(body.min_client_version).bind(id).execute(&state.pool).await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    audit(&state, admin.user_id, "connector.force_update", &id.to_string(), &json!({})).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn catalog(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>, ServerError> {
    let (user_id, _role, _epoch) = current_user(&state, &headers).await?;
    let user = uuid_of(&user_id)?;
    let rows = sqlx::query(
        "SELECT c.id, c.name, c.transport, c.force_update, c.min_client_version,
                COALESCE((SELECT can_enable FROM connector_grants g WHERE g.connector_id = c.id AND g.grantee = $1), false) AS user_can,
                COALESCE((SELECT can_enable FROM connector_grants g WHERE g.connector_id = c.id AND g.grantee = 'all'), false) AS all_can,
                COALESCE((SELECT enabled FROM connector_prefs p WHERE p.connector_id = c.id AND p.user_id = $2), false) AS enabled,
                o.client_id, o.scopes, o.redirect_uri
         FROM connectors c LEFT JOIN connector_oauth o ON o.connector_id = c.id
         WHERE c.deleted_at IS NULL AND c.transport IS NOT NULL
           AND (EXISTS (SELECT 1 FROM connector_grants g WHERE g.connector_id = c.id AND g.grantee = 'all')
                OR EXISTS (SELECT 1 FROM connector_grants g WHERE g.connector_id = c.id AND g.grantee = $1))
         ORDER BY c.name",
    )
    .bind(user.to_string()).bind(user)
    .fetch_all(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let items: Vec<Value> = rows.into_iter().map(|row| {
        let can_enable = row.try_get::<Option<bool>, _>("user_can").ok().flatten().unwrap_or(false)
            || row.try_get::<Option<bool>, _>("all_can").ok().flatten().unwrap_or(false);
        json!({
            "id": row.try_get::<Uuid, _>("id").map(|v| v.to_string()).unwrap_or_default(),
            "name": row.try_get::<String, _>("name").unwrap_or_default(),
            "transport": row.try_get::<String, _>("transport").unwrap_or_default(),
            "enabled": row.try_get::<Option<bool>, _>("enabled").ok().flatten().unwrap_or(false),
            "can_enable": can_enable,
            "force_update": row.try_get::<bool, _>("force_update").unwrap_or(false),
            "min_client_version": row.try_get::<String, _>("min_client_version").unwrap_or_default(),
            "oauth": { "client_id": row.try_get::<Option<String>, _>("client_id").ok().flatten(), "scopes": row.try_get::<Option<Value>, _>("scopes").ok().flatten().unwrap_or(json!([])), "redirect_uri": row.try_get::<Option<String>, _>("redirect_uri").ok().flatten() },
        })
    }).collect();
    Ok(Json(json!({ "ok": true, "items": items })))
}

async fn toggle(State(state): State<AppState>, headers: HeaderMap, Path((id, action)): Path<(String, String)>) -> Result<Json<Value>, ServerError> {
    let (user_id, _role, _epoch) = current_user(&state, &headers).await?;
    let user = uuid_of(&user_id)?;
    let connector = uuid_of(&id)?;
    let enabled = match action.as_str() {
        "enable" => true,
        "disable" => false,
        _ => return Err(error(StatusCode::BAD_REQUEST, "VALIDATION")),
    };
    let allowed: Option<bool> = sqlx::query_scalar(
        "SELECT COALESCE((SELECT can_enable FROM connector_grants g WHERE g.connector_id = $1 AND (g.grantee = $2 OR g.grantee = 'all') AND can_enable LIMIT 1), false)",
    ).bind(connector).bind(user.to_string()).fetch_one(&state.pool).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if allowed != Some(true) { return Err(error(StatusCode::FORBIDDEN, "CONNECTOR_FORBIDDEN")); }
    sqlx::query("INSERT INTO connector_prefs (user_id, connector_id, enabled) VALUES ($1, $2, $3) ON CONFLICT (user_id, connector_id) DO UPDATE SET enabled = EXCLUDED.enabled, updated_at = now()")
        .bind(user).bind(connector).bind(enabled).execute(&state.pool).await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(Json(json!({ "ok": true, "enabled": enabled })))
}
