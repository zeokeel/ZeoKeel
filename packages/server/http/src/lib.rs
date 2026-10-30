//! Enterprise HTTP entrypoint and PostgreSQL readiness (M2 P1).
#![forbid(unsafe_code)]

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use zk_contract_server::ServerModule;
pub use zk_kit_server::AppState;
use zk_kit_server::{
    brand_snapshot, content_etag, if_none_match, min_client_version, permissions_for_role,
    require_admin, ServerError,
};

/// Stable API version for the cross-major `/api/meta/*` surface (`docs/04` §2).
pub const API_VERSION: &str = "v1";

/// Stable module descriptor.
#[must_use]
pub fn module() -> ServerModule {
    ServerModule::new("srv/http", env!("CARGO_PKG_VERSION"))
}

/// Connect to PostgreSQL (fail-closed caller: exit if this errors).
pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(16)
        .connect(database_url)
        .await
}

/// Core migration: only the migration ledger; domains own their schema.
pub const MIGRATIONS: &[(&str, &str)] =
    &[("0001_core", include_str!("../migrations/0001_core.sql"))];

/// Build server state (fresh ephemeral JWT key).
#[must_use]
pub fn state(pool: PgPool) -> AppState {
    AppState::new(pool)
}

/// Build the server router from shared state.
pub fn router_from(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/api/meta", get(meta))
        .route("/api/meta/capabilities", get(capabilities))
        .route("/api/channel", get(channel))
        .route("/api/admin/meta", get(admin_meta))
        .with_state(state)
}

/// Build the server router with a fresh state.
pub fn router(pool: PgPool) -> Router {
    router_from(state(pool))
}

async fn healthz() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "module": module().id,
        "version": module().version,
    }))
}

async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
    {
        Ok(_) => (
            axum::http::StatusCode::OK,
            Json(json!({ "status": "ready" })),
        ),
        Err(_) => (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "not_ready" })),
        ),
    }
}

async fn meta() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "edition": zk_composed_placeholder(),
        "module": module(),
    }))
}

fn zk_composed_placeholder() -> &'static str {
    // The composed app injects the real edition; this package stays composition-neutral.
    "enterprise"
}

/// Stable capability-negotiation payload (`docs/04` §2.1 / `docs/04` §3).
fn capabilities_payload() -> Value {
    let build_channel =
        std::env::var("ZEOKEEL_BUILD_CHANNEL").unwrap_or_else(|_| "official".to_owned());
    let build_profile =
        std::env::var("ZEOKEEL_BUILD_PROFILE").unwrap_or_else(|_| "default".to_owned());
    let app_origin_scheme =
        std::env::var("ZEOKEEL_APP_ORIGIN_SCHEME").unwrap_or_else(|_| "zeokeel".to_owned());
    json!({
        "api_version": API_VERSION,
        "server_version": module().version,
        "min_client_version": min_client_version(),
        "features": [
            "auth.local",
            "gateway.responses",
            "gateway.routing",
            "wallet.ledger",
            "skills",
            "connectors",
            "apps",
            "audit.hash_chain",
            "updates.manifest",
            "admin.providers",
        ],
        "auth_methods": ["local"],
        "app_origin_scheme": app_origin_scheme,
        "event_topics": [],
        "build": {
            "edition": zk_composed_placeholder(),
            "profile": build_profile,
            "channel": build_channel,
        },
    })
}

/// Public capability negotiation with ETag/304 (`docs/04` §2.1).
async fn capabilities(headers: HeaderMap) -> Response {
    let payload = capabilities_payload();
    let rendered = serde_json::to_vec(&payload).unwrap_or_default();
    let mut hash = Sha256::new();
    hash.update(&rendered);
    let etag = format!("\"{:x}\"", hash.finalize());
    let if_none_match = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if if_none_match == etag || if_none_match == "*" {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::NOT_MODIFIED;
        if let Ok(value) = HeaderValue::from_str(&etag) {
            response.headers_mut().insert(header::ETAG, value);
        }
        return response;
    }
    let mut response = Response::new(Body::from(rendered));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    if let Ok(value) = HeaderValue::from_str(&etag) {
        response.headers_mut().insert(header::ETAG, value);
    }
    response
}

/// Brand/channel payload for the stable `/api/channel` surface (`docs/04` §2.2).
fn channel_payload() -> Value {
    let brand = brand_snapshot();
    let channel_id = brand
        .get("channel")
        .and_then(Value::as_str)
        .unwrap_or("official")
        .to_owned();
    let name = brand
        .get("name")
        .cloned()
        .unwrap_or_else(|| json!(channel_id));
    let server_url = brand
        .get("server_url")
        .cloned()
        .unwrap_or_else(|| json!(""));
    json!({
        "channel_id": channel_id,
        "name": name,
        "brand": brand.clone(),
        "server_url": server_url,
        "assets": { "base_url": "" },
        "update": {
            "mode": "server",
            "manifest": "/api/updates/manifest",
            "channel": channel_id,
        },
    })
}

/// Public channel/brand discovery with ETag/304 (`docs/04` §2.2 stable surface).
async fn channel(headers: HeaderMap) -> Response {
    let payload = channel_payload();
    let etag = content_etag(&payload);
    if if_none_match(&headers, &etag) {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::NOT_MODIFIED;
        if let Ok(value) = HeaderValue::from_str(&etag) {
            response.headers_mut().insert(header::ETAG, value);
        }
        return response;
    }
    let rendered = serde_json::to_vec(&payload).unwrap_or_default();
    let mut response = Response::new(Body::from(rendered));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    if let Ok(value) = HeaderValue::from_str(&etag) {
        response.headers_mut().insert(header::ETAG, value);
    }
    response
}

/// Admin build/permission metadata so the admin SPA can render capabilities
/// without guessing (`docs/08` admin shell; permission points `docs/04` §3).
async fn admin_meta(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    require_admin(&state, &headers, false).await?;
    Ok(Json(json!({
        "ok": true,
        "api_version": API_VERSION,
        "server_version": module().version,
        "capabilities": capabilities_payload(),
        "roles": ["super_admin", "auditor", "employee"],
        "permissions": {
            "super_admin": permissions_for_role("super_admin"),
            "auditor": permissions_for_role("auditor"),
            "employee": permissions_for_role("employee"),
        },
    })))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)] // test-only
mod tests {
    use super::*;

    #[test]
    fn module_is_namespaced() {
        assert_eq!(module().id, "srv/http");
    }

    #[test]
    fn channel_payload_has_stable_surface_fields() {
        let payload = channel_payload();
        assert!(payload.get("channel_id").and_then(Value::as_str).is_some());
        assert!(payload.get("brand").is_some_and(Value::is_object));
        assert_eq!(
            payload
                .get("update")
                .and_then(|value| value.get("mode"))
                .and_then(Value::as_str),
            Some("server")
        );
    }

    #[tokio::test]
    async fn router_builds() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://localhost/zk_never")
            .expect("lazy pool");
        let _ = router(pool);
    }
}
