//! M2 P1 admin users/roles API (no departments/user groups).
#![allow(clippy::too_many_lines)] // focused P1 slice

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, patch};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

use zk_kit_server::{error as auth_error, hash_password, require_admin, AppState, ServerError as AuthError};

const ROLES: [&str; 3] = ["super_admin", "auditor", "employee"];

#[derive(Debug, Deserialize)]
struct CreateUser {
    username: String,
    password: String,
    #[serde(default)]
    display_name: String,
    role: String,
}

#[derive(Debug, Deserialize)]
struct UpdateUser {
    role: Option<String>,
    status: Option<String>,
    password: Option<String>,
}

fn validate_role(role: &str) -> Result<(), AuthError> {
    if ROLES.contains(&role) {
        Ok(())
    } else {
        Err(auth_error(StatusCode::BAD_REQUEST, "INVALID_ROLE"))
    }
}

pub(crate) async fn last_super_admin(state: &AppState, exclude: Uuid) -> Result<bool, AuthError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM users WHERE role = 'super_admin' AND status = 'active' AND deleted_at IS NULL AND id <> $1",
    )
    .bind(exclude)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(count == 0)
}

async fn list(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<Value>, AuthError> {
    require_admin(&state, &headers, false).await?;
    let rows = sqlx::query("SELECT id, username, display_name, role, status, created_at, last_login_at FROM users WHERE deleted_at IS NULL ORDER BY created_at ASC")
        .fetch_all(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let mut users = Vec::new();
    for row in rows {
        users.push(json!({
            "id": row.try_get::<Uuid, _>("id").map(|id| id.to_string()).unwrap_or_default(),
            "username": row.try_get::<String, _>("username").unwrap_or_default(),
            "display_name": row.try_get::<String, _>("display_name").unwrap_or_default(),
            "role": row.try_get::<String, _>("role").unwrap_or_default(),
            "status": row.try_get::<String, _>("status").unwrap_or_default(),
        }));
    }
    Ok(Json(json!({ "ok": true, "users": users })))
}

async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateUser>,
) -> Result<Json<Value>, AuthError> {
    let admin = require_admin(&state, &headers, true).await?;
    if request.username.trim().is_empty() || request.password.len() < 8 {
        return Err(auth_error(StatusCode::BAD_REQUEST, "INVALID_USER"));
    }
    validate_role(&request.role)?;
    let hash = hash_password(&request.password).map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "HASH_ERROR"))?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (username, display_name, password_hash, role) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(request.username.trim())
    .bind(&request.display_name)
    .bind(hash)
    .bind(&request.role)
    .fetch_one(&state.pool)
    .await
    .map_err(|error| {
        if error.to_string().contains("duplicate") {
            auth_error(StatusCode::CONFLICT, "USERNAME_TAKEN")
        } else {
            auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR")
        }
    })?;
    sqlx::query("INSERT INTO wallet_accounts (user_id, balance_micros) VALUES ($1, 0) ON CONFLICT (user_id) DO NOTHING").bind(id).execute(&state.pool).await.map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    zk_kit_server::audit(&state, admin.user_id, "user.create", &id.to_string(), &json!({ "role": request.role })).await?;
    Ok(Json(json!({ "ok": true, "id": id.to_string() })))
}

async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateUser>,
) -> Result<Json<Value>, AuthError> {
    let admin = require_admin(&state, &headers, true).await?;
    let user_id = Uuid::parse_str(&id).map_err(|_| auth_error(StatusCode::BAD_REQUEST, "INVALID_USER"))?;
    let current = sqlx::query("SELECT role, status FROM users WHERE id = $1 AND deleted_at IS NULL")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| auth_error(StatusCode::NOT_FOUND, "USER_NOT_FOUND"))?;
    let current_role: String = current.try_get("role").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let current_status: String = current.try_get("status").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let next_role = request.role.unwrap_or_else(|| current_role.clone());
    let next_status = request.status.unwrap_or_else(|| current_status.clone());
    validate_role(&next_role)?;
    if next_status != "active" && next_status != "disabled" {
        return Err(auth_error(StatusCode::BAD_REQUEST, "INVALID_STATUS"));
    }
    if current_role == "super_admin"
        && (next_role != "super_admin" || next_status != "active")
        && last_super_admin(&state, user_id).await?
    {
        return Err(auth_error(StatusCode::CONFLICT, "LAST_SUPER_ADMIN"));
    }
    let password_hash = match request.password {
        Some(password) if password.len() >= 8 => Some(hash_password(&password).map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "HASH_ERROR"))?),
        Some(_) => return Err(auth_error(StatusCode::BAD_REQUEST, "INVALID_PASSWORD")),
        None => None,
    };
    sqlx::query(
        "UPDATE users SET role = $1, status = $2, password_hash = COALESCE($3, password_hash), token_epoch = token_epoch + 1, updated_at = now() WHERE id = $4",
    )
    .bind(&next_role)
    .bind(&next_status)
    .bind(password_hash)
    .bind(user_id)
    .execute(&state.pool)
    .await
    .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    zk_kit_server::audit(&state, admin.user_id, "user.update", &id, &json!({ "role": next_role, "status": next_status })).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, AuthError> {
    let admin = require_admin(&state, &headers, true).await?;
    let user_id = Uuid::parse_str(&id).map_err(|_| auth_error(StatusCode::BAD_REQUEST, "INVALID_USER"))?;
    let current = sqlx::query("SELECT role FROM users WHERE id = $1 AND deleted_at IS NULL")
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| auth_error(StatusCode::NOT_FOUND, "USER_NOT_FOUND"))?;
    let role: String = current.try_get("role").map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if role == "super_admin" && last_super_admin(&state, user_id).await? {
        return Err(auth_error(StatusCode::CONFLICT, "LAST_SUPER_ADMIN"));
    }
    sqlx::query("UPDATE users SET deleted_at = now(), status = 'disabled', token_epoch = token_epoch + 1 WHERE id = $1")
        .bind(user_id)
        .execute(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query("UPDATE refresh_tokens SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(user_id)
        .execute(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query("UPDATE admin_sessions SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(user_id)
        .execute(&state.pool)
        .await
        .map_err(|_| auth_error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    zk_kit_server::audit(&state, admin.user_id, "user.delete", &id, &json!({ "soft": true })).await?;
    Ok(Json(json!({ "ok": true })))
}

/// Admin user routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/users", get(list).post(create))
        .route("/api/admin/users/{id}", patch(update).delete(remove))
}
