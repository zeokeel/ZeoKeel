//! `zk-server`: enterprise thin-management companion (M2 P1).
//!
//! The server never runs an agent or skill. P1 exposes health/readiness and applies
//! the immutable schema; auth/admin/wallet/gateway land in subsequent P1–P2 slices.
#![forbid(unsafe_code)]

use axum::Router;
use zk_srv_http::{auth::ensure_bootstrap_admin, connect, router_from, run_migrations, state};

const DEFAULT_ADDR: &str = "127.0.0.1:8787";

#[tokio::main]
async fn main() -> Result<(), String> {
    let database_url = std::env::var("ZEOKEEL_DATABASE_URL")
        .map_err(|_| "ZEOKEEL_DATABASE_URL is required".to_owned())?;
    let pool = connect(&database_url)
        .await
        .map_err(|error| format!("connect PostgreSQL: {error}"))?;
    run_migrations(&pool)
        .await
        .map_err(|error| format!("apply migrations: {error}"))?;
    let state = state(pool);
    if let Ok(password) = std::env::var("ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD") {
        let username = std::env::var("ZEOKEEL_BOOTSTRAP_ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_owned());
        let created = ensure_bootstrap_admin(&state, &username, &password)
            .await
            .map_err(|error| format!("bootstrap admin: {error}"))?;
        if created {
            println!("bootstrap admin created: {username}");
        }
    }
    serve(router_from(state)).await
}

async fn serve(app: Router) -> Result<(), String> {
    let addr = std::env::var("ZEOKEEL_SERVER_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_owned());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|error| format!("bind {addr}: {error}"))?;
    println!("zk-server listening on http://{addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|error| format!("zk-server failed: {error}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn edition_is_composed() {
        assert!(!zk_composed::EDITION.is_empty());
    }
}
