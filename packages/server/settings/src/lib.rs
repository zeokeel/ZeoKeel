#![forbid(unsafe_code)]
//! M2 P3/P4 system settings and client bootstrap.
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

use zk_kit_server::AppState;
use zk_kit_server::{
    client_is_too_old, client_version, content_etag, current_user as current_user_public,
    error as auth_error, if_none_match, min_client_version_for_server, permissions_for_role,
    request_id_for_request, require_admin, upgrade_required_response, ServerError as AuthError,
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

fn config_unavailable() -> AuthError {
    auth_error(StatusCode::SERVICE_UNAVAILABLE, "CONFIG_UNAVAILABLE")
}

fn not_modified(etag: &str) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_MODIFIED;
    if let Ok(value) = axum::http::HeaderValue::from_str(etag) {
        response.headers_mut().insert(header::ETAG, value);
    }
    response
}

fn json_with_etag(body: Vec<u8>, etag: &str) -> Response {
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/json"),
    );
    if let Ok(value) = axum::http::HeaderValue::from_str(etag) {
        response.headers_mut().insert(header::ETAG, value);
    }
    response
}

async fn bootstrap(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AuthError> {
    let (user_id, role, _) = current_user_public(&state, &headers).await?;
    if client_is_too_old(&headers) {
        let request_id = request_id_for_request(&headers);
        let current = client_version(&headers);
        let minimum = min_client_version_for_server();
        return Ok(upgrade_required_response(
            &request_id,
            current.as_deref(),
            &minimum,
            "client version is below the minimum supported version",
        ));
    }
    let user = Uuid::parse_str(&user_id)
        .map_err(|_| auth_error(StatusCode::UNAUTHORIZED, "AUTH_EXPIRED"))?;

    let models = sqlx::query("SELECT model, input_per_1k_micros, output_per_1k_micros FROM model_prices WHERE enabled = true ORDER BY model")
        .fetch_all(&state.pool)
        .await
        .map_err(|_| config_unavailable())?;
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

    let skill_rows = sqlx::query(
        "SELECT c.name, r.version, r.sha256, c.min_client_version, c.force_update
         FROM contents c JOIN content_releases r ON r.id = c.current_release_id
         WHERE c.kind = 'skill' AND c.status = 'approved' AND c.deleted_at IS NULL
           AND (c.visibility = 'all'
                OR EXISTS (SELECT 1 FROM content_grants g WHERE g.content_id = c.id AND g.grantee = $1))
           AND (COALESCE((SELECT can_install FROM content_grants g WHERE g.content_id = c.id AND g.grantee = 'all'), false)
                OR COALESCE((SELECT can_install FROM content_grants g WHERE g.content_id = c.id AND g.grantee = $1), false))
         ORDER BY c.name",
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| config_unavailable())?;
    let skills: Vec<Value> = skill_rows
        .into_iter()
        .map(|row| {
            let name = row.try_get::<String, _>("name").unwrap_or_default();
            json!({
                "name": name,
                "version": row.try_get::<String, _>("version").unwrap_or_default(),
                "title": name,
                "category": "",
                "sha256": row.try_get::<String, _>("sha256").unwrap_or_default(),
                "min_client_version": row.try_get::<String, _>("min_client_version").unwrap_or_default(),
                "enforced": row.try_get::<bool, _>("force_update").unwrap_or(false),
            })
        })
        .collect();

    let connector_rows = sqlx::query(
        "SELECT c.id, c.name, c.transport, c.force_update, c.min_client_version,
                COALESCE((SELECT can_enable FROM connector_grants g WHERE g.connector_id = c.id AND g.grantee = $1), false) AS user_can,
                COALESCE((SELECT can_enable FROM connector_grants g WHERE g.connector_id = c.id AND g.grantee = 'all'), false) AS all_can,
                COALESCE((SELECT enabled FROM connector_prefs p WHERE p.connector_id = c.id AND p.user_id = $2), false) AS enabled,
                o.client_id, o.scopes, o.redirect_uri,
                c.config->>'authorize_url' AS authorize_url, c.config->>'token_url' AS token_url
         FROM connectors c LEFT JOIN connector_oauth o ON o.connector_id = c.id
         WHERE c.deleted_at IS NULL AND c.transport IS NOT NULL
           AND (EXISTS (SELECT 1 FROM connector_grants g WHERE g.connector_id = c.id AND g.grantee = 'all')
                OR EXISTS (SELECT 1 FROM connector_grants g WHERE g.connector_id = c.id AND g.grantee = $1))
         ORDER BY c.name",
    )
    .bind(&user_id)
    .bind(user)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| config_unavailable())?;
    let connectors: Vec<Value> = connector_rows
        .into_iter()
        .map(|row| {
            let oauth_client = row
                .try_get::<Option<String>, _>("client_id")
                .ok()
                .flatten();
            let can_enable = row
                .try_get::<Option<bool>, _>("user_can")
                .ok()
                .flatten()
                .unwrap_or(false)
                || row
                    .try_get::<Option<bool>, _>("all_can")
                    .ok()
                    .flatten()
                    .unwrap_or(false);
            json!({
                "id": row.try_get::<Uuid, _>("id").map(|value| value.to_string()).unwrap_or_default(),
                "name": row.try_get::<String, _>("name").unwrap_or_default(),
                "transport": row.try_get::<String, _>("transport").unwrap_or_default(),
                "auth_mode": if oauth_client.is_some() { "oauth" } else { "none" },
                "scopes": row.try_get::<Option<Value>, _>("scopes").ok().flatten().unwrap_or_else(|| json!([])),
                "enabled": row.try_get::<Option<bool>, _>("enabled").ok().flatten().unwrap_or(false),
                "can_enable": can_enable,
                "force_update": row.try_get::<bool, _>("force_update").unwrap_or(false),
                "min_client_version": row.try_get::<String, _>("min_client_version").unwrap_or_default(),
                "oauth": {
                    "client_id": oauth_client,
                    "scopes": row.try_get::<Option<Value>, _>("scopes").ok().flatten().unwrap_or_else(|| json!([])),
                    "redirect_uri": row.try_get::<Option<String>, _>("redirect_uri").ok().flatten(),
                    "authorize_url": row.try_get::<Option<String>, _>("authorize_url").ok().flatten(),
                    "token_url": row.try_get::<Option<String>, _>("token_url").ok().flatten(),
                },
            })
        })
        .collect();

    let app_rows = sqlx::query(
        "SELECT c.id, c.name, c.visibility, c.force_update, c.min_client_version, r.version, r.ui_manifest
         FROM contents c JOIN content_releases r ON r.id = c.current_release_id
         WHERE c.kind = 'app' AND c.status = 'approved' AND c.deleted_at IS NULL
           AND (c.visibility = 'all'
                OR EXISTS (SELECT 1 FROM content_grants g WHERE g.content_id = c.id AND g.grantee = $1 AND g.can_install))
         ORDER BY c.name",
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| config_unavailable())?;
    let apps: Vec<Value> = app_rows
        .into_iter()
        .map(|row| {
            let ui_manifest = row
                .try_get::<Option<Value>, _>("ui_manifest")
                .ok()
                .flatten()
                .unwrap_or(Value::Null);
            json!({
                "app_id": row.try_get::<Uuid, _>("id").map(|value| value.to_string()).unwrap_or_default(),
                "name": row.try_get::<String, _>("name").unwrap_or_default(),
                "description": "",
                "icon": "",
                "version": row.try_get::<String, _>("version").unwrap_or_default(),
                "access": row.try_get::<String, _>("visibility").unwrap_or_default(),
                "ui": !ui_manifest.is_null(),
                "force_update": row.try_get::<bool, _>("force_update").unwrap_or(false),
                "min_client_version": row.try_get::<String, _>("min_client_version").unwrap_or_default(),
            })
        })
        .collect();

    let update_channel: Option<Value> =
        sqlx::query_scalar("SELECT value FROM system_settings WHERE key = 'updates.channel'")
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| config_unavailable())?;
    let brand = zk_kit_server::brand_snapshot().clone();
    let mut payload = json!({
        "schema_version": 1,
        "identity": {
            "user_id": user_id,
            "role": role,
            "permissions": permissions_for_role(&role),
        },
        "gateway": {
            "models": catalog,
            "default_model": catalog.first().and_then(|model| model.get("id")).cloned().unwrap_or(Value::Null),
        },
        "skills": skills,
        "connectors": connectors,
        "apps": apps,
        "updates": update_channel.unwrap_or(Value::Null),
        "brand": brand,
    });
    let etag = content_etag(&payload);
    if if_none_match(&headers, &etag) {
        return Ok(not_modified(&etag));
    }
    if let Some(object) = payload.as_object_mut() {
        object.insert("config_etag".to_owned(), Value::String(etag.clone()));
    }
    let body = serde_json::to_vec(&payload).map_err(|_| config_unavailable())?;
    Ok(json_with_etag(body, &etag))
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
