//! Skill archive validation/catalog/download (`srv.skills`).
#![forbid(unsafe_code)]

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::io::{Cursor, Read};
use uuid::Uuid;
use zk_kit_content::is_safe_archive_path;
use zk_kit_server::{audit_tx, current_user, error, require_admin, AppState, ServerError};

const MAX_ARCHIVE: usize = 16 * 1024 * 1024;
const MAX_UNCOMPRESSED: u64 = 64 * 1024 * 1024;
const MAX_ENTRIES: usize = 256;

/// Skill routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/skills/{id}/versions", post(upload))
        .route("/api/client/v1/skills", post(upload_client))
        .route(
            "/api/client/v1/skills/{name}/{version}/archive",
            get(download),
        )
        .route(
            "/api/client/v1/skills/{name}/{version}/install",
            post(receipt),
        )
        .route("/api/client/v1/skills/tombstones", get(tombstones))
        .layer(DefaultBodyLimit::max(MAX_ARCHIVE))
}

#[derive(Debug)]
struct ArchiveMeta {
    sha256: String,
    size: u64,
}

fn validate_archive(bytes: &[u8]) -> Result<ArchiveMeta, ServerError> {
    if bytes.len() > MAX_ARCHIVE {
        return Err(error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "SKILL_ARCHIVE_TOO_LARGE",
        ));
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| error(StatusCode::BAD_REQUEST, "SKILL_ZIP_INVALID"))?;
    if archive.len() > MAX_ENTRIES {
        return Err(error(StatusCode::BAD_REQUEST, "SKILL_ENTRY_FORBIDDEN"));
    }
    let mut total = 0_u64;
    let mut has_skill_md = false;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|_| error(StatusCode::BAD_REQUEST, "SKILL_ZIP_INVALID"))?;
        let name = entry.name().to_owned();
        if !is_safe_archive_path(name.trim_end_matches('/')) {
            return Err(error(StatusCode::BAD_REQUEST, "SKILL_ENTRY_FORBIDDEN"));
        }
        let mode = entry.unix_mode().unwrap_or(0o100644);
        let kind = mode & 0o170000;
        if kind != 0 && kind != 0o100000 && kind != 0o040000 {
            return Err(error(StatusCode::BAD_REQUEST, "SKILL_ENTRY_FORBIDDEN"));
        }
        total = total.saturating_add(entry.size());
        if total > MAX_UNCOMPRESSED {
            return Err(error(StatusCode::BAD_REQUEST, "SKILL_ARCHIVE_TOO_LARGE"));
        }
        if name == "SKILL.md" || name.ends_with("/SKILL.md") {
            has_skill_md = true;
        }
    }
    if !has_skill_md {
        return Err(error(StatusCode::BAD_REQUEST, "SKILL_MD_MISSING"));
    }
    let mut hash = Sha256::new();
    hash.update(bytes);
    let size = u64::try_from(bytes.len())
        .map_err(|_| error(StatusCode::PAYLOAD_TOO_LARGE, "SKILL_ARCHIVE_TOO_LARGE"))?;
    Ok(ArchiveMeta {
        sha256: format!("{:x}", hash.finalize()),
        size,
    })
}

async fn read_upload(mut multipart: Multipart) -> Result<(String, String, Bytes), ServerError> {
    let mut name = String::new();
    let mut version = String::new();
    let mut bytes: Option<Bytes> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| error(StatusCode::BAD_REQUEST, "VALIDATION"))?
    {
        match field.name() {
            Some("name") => {
                name = field
                    .text()
                    .await
                    .map_err(|_| error(StatusCode::BAD_REQUEST, "VALIDATION"))?
            }
            Some("version") => {
                version = field
                    .text()
                    .await
                    .map_err(|_| error(StatusCode::BAD_REQUEST, "VALIDATION"))?
            }
            Some("archive") => {
                bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|_| error(StatusCode::BAD_REQUEST, "VALIDATION"))?,
                )
            }
            _ => {}
        }
    }
    let Some(bytes) = bytes else {
        return Err(error(StatusCode::BAD_REQUEST, "VALIDATION"));
    };
    Ok((name, version, bytes))
}

async fn upload_client(
    State(state): State<AppState>,
    headers: HeaderMap,
    multipart: Multipart,
) -> Result<Json<serde_json::Value>, ServerError> {
    let (user_id, _role, _epoch) = current_user(&state, &headers).await?;
    let actor =
        Uuid::parse_str(&user_id).map_err(|_| error(StatusCode::UNAUTHORIZED, "AUTH_EXPIRED"))?;
    let (name, version, bytes) = read_upload(multipart).await?;
    if name.trim().is_empty() || version.trim().is_empty() {
        return Err(error(StatusCode::BAD_REQUEST, "VALIDATION"));
    }
    let meta = validate_archive(&bytes)?;
    // Upload + release + audit are one transaction (fail-closed audit).
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let existing: Option<(Uuid, Uuid)> = sqlx::query_as("SELECT id, owner_id FROM contents WHERE kind = 'skill' AND name = $1 AND deleted_at IS NULL")
        .bind(name.trim()).fetch_optional(&mut *tx).await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let content = match existing {
        Some((content, owner)) => {
            if owner != actor {
                return Err(error(StatusCode::FORBIDDEN, "SKILL_FORBIDDEN"));
            }
            content
        }
        None => {
            let content: Uuid = sqlx::query_scalar("INSERT INTO contents (kind, name, owner_id, status) VALUES ('skill', $1, $2, 'pending') RETURNING id")
                .bind(name.trim()).bind(actor).fetch_one(&mut *tx).await
                .map_err(|_| error(StatusCode::CONFLICT, "CONFLICT"))?;
            content
        }
    };
    let release: Uuid = sqlx::query_scalar(
        "INSERT INTO content_releases (content_id, version, sha256, archive_size, archive, changelog, created_by)
         VALUES ($1, $2, $3, $4, $5, '', $6) RETURNING id",
    ).bind(content).bind(version.trim()).bind(&meta.sha256)
        .bind(i64::try_from(meta.size).map_err(|_| error(StatusCode::PAYLOAD_TOO_LARGE, "SKILL_ARCHIVE_TOO_LARGE"))?)
        .bind(bytes.as_ref()).bind(actor).fetch_one(&mut *tx).await
        .map_err(|_| error(StatusCode::CONFLICT, "CONFLICT"))?;
    sqlx::query("UPDATE contents SET current_release_id = $1, status = 'pending', updated_at = now() WHERE id = $2")
        .bind(release).bind(content).execute(&mut *tx).await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    audit_tx(
        &mut tx,
        actor,
        "skill.upload",
        &content.to_string(),
        &json!({ "name": name.trim(), "version": version.trim(), "sha256": meta.sha256 }),
    )
    .await?;
    tx.commit()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(Json(
        json!({ "ok": true, "id": content.to_string(), "release_id": release.to_string(), "status": "pending" }),
    ))
}

async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, ServerError> {
    let admin = require_admin(&state, &headers, true).await?;
    let content_id =
        Uuid::parse_str(&id).map_err(|_| error(StatusCode::BAD_REQUEST, "VALIDATION"))?;
    let mut version = String::new();
    let mut bytes: Option<Bytes> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| error(StatusCode::BAD_REQUEST, "VALIDATION"))?
    {
        match field.name() {
            Some("version") => {
                version = field
                    .text()
                    .await
                    .map_err(|_| error(StatusCode::BAD_REQUEST, "VALIDATION"))?
            }
            Some("archive") => {
                bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|_| error(StatusCode::BAD_REQUEST, "VALIDATION"))?,
                )
            }
            _ => {}
        }
    }
    if version.trim().is_empty() || version.len() > 64 {
        return Err(error(StatusCode::BAD_REQUEST, "VALIDATION"));
    }
    let Some(bytes) = bytes else {
        return Err(error(StatusCode::BAD_REQUEST, "VALIDATION"));
    };
    let meta = validate_archive(&bytes)?;
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let owner: Option<Uuid> = sqlx::query_scalar(
        "SELECT owner_id FROM contents WHERE id = $1 AND deleted_at IS NULL AND kind = 'skill'",
    )
    .bind(content_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    if owner.is_none() {
        return Err(error(StatusCode::NOT_FOUND, "SKILL_NOT_FOUND"));
    }
    let release_id: Uuid = sqlx::query_scalar(
        "INSERT INTO content_releases (content_id, version, sha256, archive_size, archive, changelog, created_by)
         VALUES ($1, $2, $3, $4, $5, '', $6) RETURNING id",
    )
    .bind(content_id)
    .bind(version.trim())
    .bind(&meta.sha256)
    .bind(i64::try_from(meta.size).map_err(|_| error(StatusCode::PAYLOAD_TOO_LARGE, "SKILL_ARCHIVE_TOO_LARGE"))?)
    .bind(bytes.as_ref())
    .bind(admin.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| error(StatusCode::CONFLICT, "CONFLICT"))?;
    sqlx::query("UPDATE contents SET current_release_id = $1, updated_at = now() WHERE id = $2")
        .bind(release_id)
        .bind(content_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    audit_tx(
        &mut tx,
        admin.user_id,
        "skill.release.create",
        &release_id.to_string(),
        &json!({ "content_id": content_id.to_string(), "version": version.trim(), "sha256": meta.sha256 }),
    )
    .await?;
    tx.commit()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(Json(
        json!({ "ok": true, "release_id": release_id.to_string(), "sha256": meta.sha256 }),
    ))
}

#[derive(Debug, serde::Deserialize)]
struct Receipt {
    tree_sha256: String,
}

async fn receipt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((name, version)): Path<(String, String)>,
    Json(body): Json<Receipt>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let (user_id, _role, _epoch) = current_user(&state, &headers).await?;
    let user =
        Uuid::parse_str(&user_id).map_err(|_| error(StatusCode::UNAUTHORIZED, "AUTH_EXPIRED"))?;
    if body.tree_sha256.len() != 64 || !body.tree_sha256.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(error(StatusCode::BAD_REQUEST, "VALIDATION"));
    }
    let row = sqlx::query(
        "SELECT c.id AS content_id, r.id AS release_id FROM contents c JOIN content_releases r ON r.content_id = c.id
         WHERE c.kind = 'skill' AND c.name = $1 AND r.version = $2 AND c.status = 'approved' AND c.deleted_at IS NULL
           AND (COALESCE((SELECT can_install FROM content_grants g WHERE g.content_id = c.id AND g.grantee = 'all'), false)
                OR COALESCE((SELECT can_install FROM content_grants g WHERE g.content_id = c.id AND g.grantee = $3), false))
         LIMIT 1",
    ).bind(&name).bind(&version).bind(user.to_string()).fetch_optional(&state.pool).await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "SKILL_NOT_FOUND"))?;
    let content: Uuid = row
        .try_get("content_id")
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let release: Uuid = row
        .try_get("release_id")
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    sqlx::query(
        "INSERT INTO content_installs (user_id, content_id, release_id, tree_sha256, source)
         VALUES ($1, $2, $3, $4, 'client')
         ON CONFLICT (user_id, content_id, release_id) DO UPDATE SET tree_sha256 = EXCLUDED.tree_sha256, installed_at = now()",
    ).bind(user).bind(content).bind(release).bind(&body.tree_sha256).execute(&mut *tx).await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    audit_tx(
        &mut tx,
        user,
        "skill.install",
        &format!("{name}@{version}"),
        &json!({ "tree_sha256": body.tree_sha256, "source": "client" }),
    )
    .await?;
    tx.commit()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    Ok(Json(json!({ "ok": true })))
}

async fn tombstones(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ServerError> {
    let (user_id, _role, _epoch) = current_user(&state, &headers).await?;
    let _user =
        Uuid::parse_str(&user_id).map_err(|_| error(StatusCode::UNAUTHORIZED, "AUTH_EXPIRED"))?;
    let rows =
        sqlx::query("SELECT name FROM contents WHERE kind = 'skill' AND deleted_at IS NOT NULL")
            .fetch_all(&state.pool)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let items: Vec<String> = rows
        .into_iter()
        .filter_map(|row| row.try_get::<String, _>("name").ok())
        .collect();
    Ok(Json(json!({ "ok": true, "items": items })))
}

async fn download(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((name, version)): Path<(String, String)>,
) -> Result<Response, ServerError> {
    let (user_id, _role, _epoch) = current_user(&state, &headers).await?;
    let user =
        Uuid::parse_str(&user_id).map_err(|_| error(StatusCode::UNAUTHORIZED, "AUTH_EXPIRED"))?;
    let row = sqlx::query(
        "SELECT r.archive, r.sha256 FROM contents c JOIN content_releases r ON r.content_id = c.id
         WHERE c.kind = 'skill' AND c.name = $1 AND r.version = $2 AND c.status = 'approved' AND c.deleted_at IS NULL
           AND (c.visibility = 'all' OR EXISTS (SELECT 1 FROM content_grants g WHERE g.content_id = c.id AND g.grantee = $3))
           AND (COALESCE((SELECT can_install FROM content_grants g WHERE g.content_id = c.id AND g.grantee = 'all'), false)
                OR COALESCE((SELECT can_install FROM content_grants g WHERE g.content_id = c.id AND g.grantee = $3), false))
         LIMIT 1",
    )
    .bind(&name)
    .bind(&version)
    .bind(user.to_string())
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?
    .ok_or_else(|| error(StatusCode::NOT_FOUND, "SKILL_NOT_FOUND"))?;
    let archive: Vec<u8> = row
        .try_get("archive")
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let sha: String = row
        .try_get("sha256")
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?;
    let mut response = archive.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/zip"),
    );
    response.headers_mut().insert(
        "x-zeokeel-skill-sha256",
        HeaderValue::from_str(&sha)
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"))?,
    );
    Ok(response)
}

/// Read one file from a validated archive (server-side tests only; never executes content).
pub fn read_entry(bytes: &[u8], path: &str) -> Result<Option<Vec<u8>>, ServerError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| error(StatusCode::BAD_REQUEST, "SKILL_ZIP_INVALID"))?;
    let Ok(mut entry) = archive.by_name(path) else {
        return Ok(None);
    };
    let mut out = Vec::new();
    entry
        .read_to_end(&mut out)
        .map_err(|_| error(StatusCode::BAD_REQUEST, "SKILL_ZIP_INVALID"))?;
    Ok(Some(out))
}
