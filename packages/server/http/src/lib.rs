//! Enterprise HTTP entrypoint and PostgreSQL readiness (M2 P1).
#![forbid(unsafe_code)]

use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use zk_contract_server::ServerModule;
pub use zk_kit_server::AppState;

/// Stable module descriptor.
#[must_use]
pub fn module() -> ServerModule {
    ServerModule::new("srv/http", env!("CARGO_PKG_VERSION"))
}

/// Connect to PostgreSQL (fail-closed caller: exit if this errors).
pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new().max_connections(16).connect(database_url).await
}

/// Core migration: only the migration ledger; domains own their schema.
pub const MIGRATIONS: &[(&str, &str)] = &[("0001_core", include_str!("../migrations/0001_core.sql"))];

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
    match sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.pool).await {
        Ok(_) => (axum::http::StatusCode::OK, Json(json!({ "status": "ready" }))),
        Err(_) => (axum::http::StatusCode::SERVICE_UNAVAILABLE, Json(json!({ "status": "not_ready" }))),
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)] // test-only
mod tests {
    use super::*;

    #[test]
    fn module_is_namespaced() {
        assert_eq!(module().id, "srv/http");
    }

    #[tokio::test]
    async fn router_builds() {
        let pool = PgPoolOptions::new().connect_lazy("postgres://localhost/zk_never").expect("lazy pool");
        let _ = router(pool);
    }
}
