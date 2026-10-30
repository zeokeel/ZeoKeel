//! Enterprise HTTP entrypoint and PostgreSQL readiness (M2 P1).
#![forbid(unsafe_code)]

use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use zk_contract_server::ServerModule;

pub mod auth;
pub mod users;
pub mod wallet;
pub mod pricing;
pub mod gateway;

/// Stable module descriptor.
#[must_use]
pub fn module() -> ServerModule {
    ServerModule::new("srv/http", env!("CARGO_PKG_VERSION"))
}

/// Connect to PostgreSQL (fail-closed caller: exit if this errors).
pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new().max_connections(16).connect(database_url).await
}

/// Apply the immutable M1 schema and record its checksum.
///
/// The SQL file is embedded; history edits are forbidden (add a new file instead).
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    let migrations: [(&str, &str); 2] = [
        ("0001_init", include_str!("../migrations/0001_init.sql")),
        ("0002_gateway", include_str!("../migrations/0002_gateway.sql")),
    ];
    for (id, sql) in migrations {
        let mut hash = Sha256::new();
        hash.update(sql.as_bytes());
        let checksum = format!("{:x}", hash.finalize());
        sqlx::raw_sql(sql).execute(pool).await?;
        sqlx::query("INSERT INTO schema_migrations (id, checksum) VALUES ($1, $2) ON CONFLICT (id) DO NOTHING")
            .bind(id)
            .bind(checksum)
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// Shared server state (P1: pool + JWT keys).
#[derive(Clone)]
pub struct AppState {
    /// PostgreSQL pool.
    pub pool: PgPool,
    /// EdDSA signing/verification material.
    pub jwt: auth::JwtKeys,
}

impl AppState {
    /// Build state with a fresh ephemeral JWT key.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool, jwt: auth::JwtKeys::ephemeral() }
    }

    #[cfg(test)]
    #[allow(clippy::expect_used)] // test-only
    fn new_for_test() -> Self {
        let pool = PgPoolOptions::new().connect_lazy("postgres://localhost/zk_never").expect("lazy pool");
        Self::new(pool)
    }
}

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
        .merge(auth::routes())
        .merge(users::routes())
        .merge(wallet::routes())
        .merge(pricing::routes())
        .merge(gateway::routes())
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
