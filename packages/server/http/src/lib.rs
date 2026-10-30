//! Enterprise HTTP entrypoint and PostgreSQL readiness (M2 P1).
#![forbid(unsafe_code)]

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
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
    attach_request_id, brand_snapshot, client_version, content_etag, if_none_match,
    min_client_version, min_client_version_for_server, parse_semver, permissions_for_role,
    request_id_for_request, request_id_from_headers, require_admin, upgrade_required_response,
    with_request_id, ServerError,
};

/// Stable API version for the cross-major `/api/meta/*` surface (`docs/04` §2).
pub const API_VERSION: &str = "v1";

/// Whether `path` belongs to a version-gated surface.
///
/// Business `/api/client/v1/*`, with login/refresh/bootstrap exempt so an old
/// client can authenticate and discover the upgrade path; gateway `/v1/*` is
/// also gated (enterprise native gateway sends the client identity header).
/// Stable surfaces (`/api/meta/*`, `/api/channel`, `/api/updates/*`) are outside.
fn is_client_business_path(path: &str) -> bool {
    if path == "/v1" || path.starts_with("/v1/") {
        return true;
    }
    if path != "/api/client/v1" && !path.starts_with("/api/client/v1/") {
        return false;
    }
    !matches!(
        path,
        "/api/client/v1/auth/login"
            | "/api/client/v1/auth/refresh"
            | "/api/client/v1/config/bootstrap"
    )
}

/// Outermost request-id middleware: read or generate exactly once, install the
/// task-local for gate/bootstrap/updates/gateway/audit, and echo the response
/// header for every wrapped response.
pub async fn request_context(request: Request, next: Next) -> Response {
    let request_id = request_id_from_headers(request.headers());
    let mut response = with_request_id(request_id.clone(), next.run(request)).await;
    attach_request_id(&mut response, &request_id);
    response
}

/// Gate `/api/client/v1/*` and `/v1/*` on the `X-ZeoKeel-Client` semver.
///
/// Missing or malformed headers fail closed with 426. The request id comes from
/// the outermost [`request_context`] task-local (no second generation); the
/// minimum is resolved from the authoritative server channel, never from the
/// client-controlled channel segment.
pub async fn client_version_gate(request: Request, next: Next) -> Response {
    // CORS preflights never carry custom request headers; let the CORS layer
    // answer them, otherwise a browser client cannot reach the business surface.
    if request.method() == axum::http::Method::OPTIONS
        || !is_client_business_path(request.uri().path())
    {
        return next.run(request).await;
    }
    let request_id = request_id_for_request(request.headers());
    let minimum = min_client_version_for_server();
    let current = client_version(request.headers());
    let Some(version) = current.as_deref().and_then(parse_semver) else {
        return upgrade_required_response(
            &request_id,
            current.as_deref(),
            &minimum,
            "X-ZeoKeel-Client header is missing or invalid",
        );
    };
    let Some(minimum_version) = parse_semver(&minimum) else {
        // The kit helper validates env overrides and falls back to 0.1.0.
        return next.run(request).await;
    };
    if version < minimum_version {
        return upgrade_required_response(
            &request_id,
            current.as_deref(),
            &minimum,
            "client version is below the minimum supported version",
        );
    }
    next.run(request).await
}

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
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::indexing_slicing)] // test-only
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn gate_router() -> Router {
        Router::new()
            .route("/api/client/v1/workspaces", get(|| async { "ok" }))
            .route("/v1/models", get(|| async { "models" }))
            .route("/v1/responses", get(|| async { "responses" }))
            .route("/api/client/v1/auth/login", get(|| async { "login" }))
            .route("/api/client/v1/auth/refresh", get(|| async { "refresh" }))
            .route(
                "/api/client/v1/config/bootstrap",
                get(|| async { "bootstrap" }),
            )
            .route("/api/meta/capabilities", get(|| async { "meta" }))
            .route("/api/channel", get(|| async { "channel" }))
            .route("/api/updates/manifest", get(|| async { "updates" }))
            .layer(axum::middleware::from_fn(client_version_gate))
            .layer(axum::middleware::from_fn(request_context))
    }

    async fn call_raw(
        app: Router,
        path: &str,
        header: Option<&str>,
        request_id: Option<&str>,
    ) -> (StatusCode, HeaderMap, Vec<u8>) {
        let mut builder = Request::builder().uri(path);
        if let Some(value) = header {
            builder = builder.header("x-zeokeel-client", value);
        }
        if let Some(value) = request_id {
            builder = builder.header("x-request-id", value);
        }
        let request = builder.body(Body::empty()).expect("request");
        let response = app.oneshot(request).await.expect("response");
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        (status, headers, bytes.to_vec())
    }

    /// Serialize tests that mutate the process-wide minimum-version env.
    fn gate_test_lock() -> &'static tokio::sync::Mutex<()> {
        static LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
    }

    async fn call_with_request_id(
        app: Router,
        path: &str,
        header: Option<&str>,
        request_id: Option<&str>,
    ) -> (StatusCode, HeaderMap, Vec<u8>) {
        let _guard = gate_test_lock().lock().await;
        call_raw(app, path, header, request_id).await
    }

    async fn call(
        app: Router,
        path: &str,
        header: Option<&str>,
    ) -> (StatusCode, HeaderMap, Vec<u8>) {
        call_with_request_id(app, path, header, None).await
    }

    #[tokio::test]
    async fn preflight_is_not_gated() {
        let request = Request::builder()
            .method("OPTIONS")
            .uri("/api/client/v1/workspaces")
            .body(Body::empty())
            .expect("request");
        let response = gate_router().oneshot(request).await.expect("response");
        assert_ne!(response.status(), StatusCode::UPGRADE_REQUIRED);
    }

    fn error_code(body: &[u8]) -> String {
        let value: Value = serde_json::from_slice(body).expect("json error body");
        value["error"]["code"]
            .as_str()
            .expect("error code")
            .to_owned()
    }

    #[tokio::test]
    async fn old_client_is_rejected_on_business_surface_with_json_426() {
        let minimum = min_client_version();
        assert_ne!(minimum, "0.0.0", "test assumes a positive minimum");
        let (status, headers, body) = call(
            gate_router(),
            "/api/client/v1/workspaces",
            Some("0.0.0;official;linux-x64;test"),
        )
        .await;
        assert_eq!(status, StatusCode::UPGRADE_REQUIRED);
        assert_eq!(error_code(&body), "CLIENT_TOO_OLD");
        let value: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(value["error"]["retryable"], false);
        assert_eq!(value["error"]["current"], "0.0.0");
        assert_eq!(value["error"]["minimum"], minimum);
        assert_eq!(value["error"]["portal_url"], "/portal");
        let request_id = value["error"]["request_id"].as_str().expect("request id");
        assert!(!request_id.is_empty());
        assert_eq!(
            headers
                .get("x-request-id")
                .and_then(|value| value.to_str().ok()),
            Some(request_id)
        );
    }

    #[tokio::test]
    async fn incoming_request_id_is_echoed_in_header_and_body() {
        let (status, headers, body) = call_with_request_id(
            gate_router(),
            "/api/client/v1/workspaces",
            Some("0.0.0;official;linux-x64;test"),
            Some("req-round6-test"),
        )
        .await;
        assert_eq!(status, StatusCode::UPGRADE_REQUIRED);
        assert_eq!(
            headers
                .get("x-request-id")
                .and_then(|value| value.to_str().ok()),
            Some("req-round6-test")
        );
        let value: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(value["error"]["request_id"], "req-round6-test");
    }

    #[tokio::test]
    async fn current_client_passes_business_surface() {
        let minimum = min_client_version();
        let header = format!("{minimum};official;linux-x64;test");
        let (status, _headers, _body) =
            call(gate_router(), "/api/client/v1/workspaces", Some(&header)).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn missing_or_invalid_header_fails_closed_with_426() {
        for header in [None, Some("not-semver;official;linux-x64;test")] {
            let (status, headers, body) =
                call(gate_router(), "/api/client/v1/workspaces", header).await;
            assert_eq!(status, StatusCode::UPGRADE_REQUIRED, "{header:?}");
            assert_eq!(error_code(&body), "CLIENT_TOO_OLD", "{header:?}");
            assert!(
                headers.get("x-request-id").is_some(),
                "request id must be echoed for {header:?}"
            );
        }
    }

    #[tokio::test]
    async fn spoofed_header_channel_cannot_lower_the_authoritative_minimum() {
        let guard = gate_test_lock().lock().await;
        let server_channel = zk_kit_server::server_channel();
        let server_key = zk_kit_server::channel_min_version_env_key(&server_channel);
        let spoof_key = zk_kit_server::channel_min_version_env_key("beta");
        std::env::set_var(&server_key, "9.9.9");
        std::env::set_var(&spoof_key, "0.0.1");
        let spoofed = "0.1.0;beta;linux-x64;test";
        let result = call_raw(
            gate_router(),
            "/api/client/v1/workspaces",
            Some(spoofed),
            None,
        )
        .await;
        std::env::remove_var(&server_key);
        std::env::remove_var(&spoof_key);
        drop(guard);

        let (status, _headers, body) = result;
        assert_eq!(status, StatusCode::UPGRADE_REQUIRED);
        let value: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(value["error"]["minimum"], "9.9.9");
    }

    #[tokio::test]
    async fn login_refresh_and_bootstrap_are_exempt() {
        for path in [
            "/api/client/v1/auth/login",
            "/api/client/v1/auth/refresh",
            "/api/client/v1/config/bootstrap",
        ] {
            for header in [None, Some("0.0.0;official;linux-x64;test")] {
                let (status, _headers, _body) = call(gate_router(), path, header).await;
                assert_eq!(status, StatusCode::OK, "{path} {header:?}");
            }
        }
    }

    #[tokio::test]
    async fn stable_surfaces_are_not_version_gated() {
        for path in [
            "/api/meta/capabilities",
            "/api/channel",
            "/api/updates/manifest",
        ] {
            for header in [None, Some("0.0.0;official;linux-x64;test")] {
                let (status, _headers, _body) = call(gate_router(), path, header).await;
                assert_eq!(status, StatusCode::OK, "{path} {header:?}");
            }
        }
    }

    #[tokio::test]
    async fn gateway_paths_are_version_gated_too() {
        for path in ["/v1/models", "/v1/responses"] {
            let (status, _headers, body) =
                call(gate_router(), path, Some("0.0.0;official;linux-x64;test")).await;
            assert_eq!(status, StatusCode::UPGRADE_REQUIRED, "{path}");
            assert_eq!(error_code(&body), "CLIENT_TOO_OLD", "{path}");
            let (status, _headers, _body) =
                call(gate_router(), path, Some("9.9.9;official;linux-x64;test")).await;
            assert_eq!(status, StatusCode::OK, "{path}");
            let (status, _headers, body) = call(gate_router(), path, None).await;
            assert_eq!(status, StatusCode::UPGRADE_REQUIRED, "{path}");
            assert_eq!(error_code(&body), "CLIENT_TOO_OLD", "{path}");
        }
    }

    #[tokio::test]
    async fn request_id_is_generated_once_and_shared_with_gate_body() {
        let (status, headers, body) = call(
            gate_router(),
            "/api/client/v1/workspaces",
            Some("0.0.0;official;linux-x64;test"),
        )
        .await;
        assert_eq!(status, StatusCode::UPGRADE_REQUIRED);
        let value: Value = serde_json::from_slice(&body).expect("json");
        let body_id = value["error"]["request_id"].as_str().expect("body id");
        assert_eq!(
            headers
                .get("x-request-id")
                .and_then(|value| value.to_str().ok()),
            Some(body_id)
        );
    }

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
