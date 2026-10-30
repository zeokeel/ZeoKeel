//! Content aggregate core (`srv.content`): status/releases/grants/deployments.
#![forbid(unsafe_code)]

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;
use zk_contract_content::{ContentGrant, ContentKind, ContentStatus, Visibility};
use zk_kit_server::{audit, current_user, error, require_admin, AppState, ServerError};

/// Immutable content migrations owned by this package.
pub const MIGRATIONS: &[(&str, &str)] = &[(
    "0010_content",
    include_str!("../migrations/0010_content.sql"),
)];

/// Content routes (M3 slice 3).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/skills", get(list_admin).post(create))
        .route("/api/admin/skills/{id}", delete(delete_content))
        .route("/api/admin/skills/{id}/approve", post(approve))
        .route("/api/admin/skills/{id}/reject", post(reject))
        .route("/api/admin/skills/{id}/delist", post(delist))
        .route("/api/admin/skills/{id}/transfer", post(transfer))
        .route("/api/admin/skills/{id}/grants", put(set_grants))
        .route("/api/admin/skills/{id}/force-update", post(force_update))
        .route("/api/client/v1/skills", get(catalog))
}

#[derive(Debug, Deserialize)]
struct CreateContent {
    name: String,
    #[serde(default)]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct TransferRequest {
    user_id: String,
}

#[derive(Debug, Deserialize)]
struct ForceUpdateRequest {
    #[serde(default = "default_true")]
    force: bool,
    #[serde(default)]
    min_client_version: String,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct GrantsRequest {
    grants: Vec<ContentGrant>,
}

fn uuid_of(value: &str) -> Result<Uuid, ServerError> {
    Uuid::parse_str(value).map_err(|_| error(StatusCode::BAD_REQUEST, "VALIDATION"))
}

async fn list_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ServerError> {
    require_admin(&state, &headers, false).await?;
    let rows = sqlx::query(
        "SELECT c.id, c.kind, c.name, c.owner_id, c.status, c.visibility, c.current_release_id, c.force_update,
                c.min_client_version, r.version
         FROM contents c LEFT JOIN content_releases r ON r.id = c.current_release_id
         WHERE c.kind = 'skill' AND c.deleted_at IS NULL ORDER BY c.created_at DESC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| json!({
            "id": row.try_get::<Uuid, _>("id").map(|value| value.to_string()).unwrap_or_default(),
            "kind": row.try_get::<String, _>("kind").unwrap_or_default(),
            "name": row.try_get::<String, _>("name").unwrap_or_default(),
            "owner_id": row.try_get::<Uuid, _>("owner_id").map(|value| value.to_string()).unwrap_or_default(),
            "status": row.try_get::<String, _>("status").unwrap_or_default(),
            "visibility": row.try_get::<String, _>("visibility").unwrap_or_default(),
            "current_release_id": row.try_get::<Option<Uuid>, _>("current_release_id").ok().flatten().map(|value| value.to_string()),
            "force_update": row.try_get::<bool, _>("force_update").unwrap_or(false),
            "min_client_version": row.try_get::<String, _>("min_client_version").unwrap_or_default(),
            "version": row.try_get::<Option<String>, _>("version").ok().flatten().unwrap_or_default(),
        }))
        .collect();
    Ok(Json(json!({ "ok": true, "items": items })))
}

async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateContent>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    let name = body.name.trim();
    if name.is_empty() || name.len() > 128 {
        return Err(error(StatusCode::BAD_REQUEST, "VALIDATION"));
    }
    let kind = if body.kind.is_empty() {
        "skill"
    } else {
        body.kind.as_str()
    };
    if kind != "skill" {
        return Err(error(StatusCode::BAD_REQUEST, "VALIDATION"));
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO contents (kind, name, owner_id, status) VALUES ('skill', $1, $2, 'pending') RETURNING id",
    )
    .bind(name)
    .bind(admin.user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| error(StatusCode::CONFLICT, "CONFLICT"))?;
    audit(
        &state,
        admin.user_id,
        "skill.upload",
        &id.to_string(),
        &json!({ "name": name }),
    )
    .await?;
    Ok(Json(json!({ "ok": true, "id": id.to_string() })))
}

async fn update_status(
    state: &AppState,
    id: Uuid,
    status: ContentStatus,
    action: &str,
    actor: Uuid,
) -> Result<Json<serde_json::Value>, ServerError> {
    let status_text = match status {
        ContentStatus::Pending => "pending",
        ContentStatus::Approved => "approved",
        ContentStatus::Rejected => "rejected",
        ContentStatus::Delisted => "delisted",
    };
    let result = sqlx::query("UPDATE contents SET status = $1, updated_at = now() WHERE id = $2 AND deleted_at IS NULL AND kind = 'skill'")
        .bind(status_text)
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if result.rows_affected() == 0 {
        return Err(error(StatusCode::NOT_FOUND, "SKILL_NOT_FOUND"));
    }
    audit(
        state,
        actor,
        action,
        &id.to_string(),
        &json!({ "status": status_text }),
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}

async fn approve(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    update_status(
        &state,
        uuid_of(&id)?,
        ContentStatus::Approved,
        "skill.approve",
        admin.user_id,
    )
    .await
}
async fn reject(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    update_status(
        &state,
        uuid_of(&id)?,
        ContentStatus::Rejected,
        "skill.reject",
        admin.user_id,
    )
    .await
}
async fn delist(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    update_status(
        &state,
        uuid_of(&id)?,
        ContentStatus::Delisted,
        "skill.delist",
        admin.user_id,
    )
    .await
}

async fn delete_content(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    let id = uuid_of(&id)?;
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let result = sqlx::query("UPDATE contents SET deleted_at = now(), updated_at = now() WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if result.rows_affected() == 0 {
        return Err(error(StatusCode::NOT_FOUND, "SKILL_NOT_FOUND"));
    }
    sqlx::query("INSERT INTO content_deployments (content_id, release_id, action, actor_id, reason) VALUES ($1, NULL, 'delete', $2, '')")
        .bind(id)
        .bind(admin.user_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    tx.commit()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    audit(
        &state,
        admin.user_id,
        "skill.delete",
        &id.to_string(),
        &json!({}),
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}

async fn transfer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<TransferRequest>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    let id = uuid_of(&id)?;
    let owner = uuid_of(&body.user_id)?;
    let result = sqlx::query("UPDATE contents SET owner_id = $1, updated_at = now() WHERE id = $2 AND deleted_at IS NULL")
        .bind(owner)
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if result.rows_affected() == 0 {
        return Err(error(StatusCode::NOT_FOUND, "SKILL_NOT_FOUND"));
    }
    audit(
        &state,
        admin.user_id,
        "skill.transfer",
        &id.to_string(),
        &json!({ "owner_id": owner.to_string() }),
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}

async fn set_grants(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<GrantsRequest>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    let id = uuid_of(&id)?;
    if body.grants.is_empty() {
        return Err(error(StatusCode::BAD_REQUEST, "VALIDATION"));
    }
    let visibility = if body.grants.iter().any(|grant| grant.grantee == "all") {
        Visibility::All
    } else {
        Visibility::Selected
    };
    let visibility_text = if visibility == Visibility::All {
        "all"
    } else {
        "selected"
    };
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query("DELETE FROM content_grants WHERE content_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    for grant in &body.grants {
        sqlx::query(
            "INSERT INTO content_grants (content_id, grantee, can_install) VALUES ($1, $2, $3)",
        )
        .bind(id)
        .bind(&grant.grantee)
        .bind(grant.can_install)
        .execute(&mut *tx)
        .await
        .map_err(|_| error(StatusCode::BAD_REQUEST, "VALIDATION"))?;
    }
    sqlx::query("UPDATE contents SET visibility = $1, updated_at = now() WHERE id = $2")
        .bind(visibility_text)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    tx.commit()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    audit(
        &state,
        admin.user_id,
        "skill.grants.update",
        &id.to_string(),
        &json!({ "visibility": visibility_text }),
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}

async fn force_update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<ForceUpdateRequest>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    let id = uuid_of(&id)?;
    let result = sqlx::query("UPDATE contents SET force_update = $1, min_client_version = $2, updated_at = now() WHERE id = $3 AND deleted_at IS NULL")
        .bind(body.force)
        .bind(&body.min_client_version)
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if result.rows_affected() == 0 {
        return Err(error(StatusCode::NOT_FOUND, "SKILL_NOT_FOUND"));
    }
    audit(
        &state,
        admin.user_id,
        "skill.force_update",
        &id.to_string(),
        &json!({ "min_client_version": body.min_client_version }),
    )
    .await?;
    Ok(Json(json!({ "ok": true })))
}

async fn catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ServerError> {
    let (user_id, _role, _epoch) = current_user(&state, &headers).await?;
    let _user = uuid_of(&user_id)?;
    let rows = sqlx::query(
        "SELECT c.id, c.name, c.visibility, r.version, r.sha256, c.force_update, c.min_client_version,
                COALESCE((SELECT can_install FROM content_grants g WHERE g.content_id = c.id AND g.grantee = $1), false) AS user_can,
                COALESCE((SELECT can_install FROM content_grants g WHERE g.content_id = c.id AND g.grantee = 'all'), false) AS all_can
         FROM contents c LEFT JOIN content_releases r ON r.id = c.current_release_id
         WHERE c.kind = 'skill' AND c.status = 'approved' AND c.deleted_at IS NULL
           AND (c.visibility = 'all' OR EXISTS (SELECT 1 FROM content_grants g WHERE g.content_id = c.id AND g.grantee = $1))
         ORDER BY c.name",
    )
    .bind(_user.to_string())
    .fetch_all(&state.pool)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let items: Vec<serde_json::Value> = rows.into_iter().map(|row| {
        let can_install = row.try_get::<Option<bool>, _>("user_can").ok().flatten().unwrap_or(false)
            || row.try_get::<Option<bool>, _>("all_can").ok().flatten().unwrap_or(false);
        json!({
            "id": row.try_get::<Uuid, _>("id").map(|value| value.to_string()).unwrap_or_default(),
            "name": row.try_get::<String, _>("name").unwrap_or_default(),
            "version": row.try_get::<Option<String>, _>("version").ok().flatten().unwrap_or_default(),
            "sha256": row.try_get::<Option<String>, _>("sha256").ok().flatten().unwrap_or_default(),
            "enforced": row.try_get::<bool, _>("force_update").unwrap_or(false),
            "min_client_version": row.try_get::<String, _>("min_client_version").unwrap_or_default(),
            "can_install": can_install,
        })
    }).collect();
    Ok(Json(json!({ "ok": true, "items": items })))
}

/// Keep the contract enums/dto referenced for future handlers.
#[allow(dead_code)]
fn _contract_types(_kind: ContentKind) {}
