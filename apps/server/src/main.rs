//! `zk-server`: enterprise thin-management companion (M2 P1).
//!
//! The server never runs an agent or skill. It applies the immutable schema
//! (fail-closed checksum verification), takes a single-instance PostgreSQL
//! advisory lock, and on SIGTERM drains requests, compensates pending usage and
//! verifies the audit chain before closing the pool (`docs/24` §5–§6).
#![forbid(unsafe_code)]

use std::future::IntoFuture;
use std::time::Duration;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use zk_kit_server::{ensure_bootstrap_admin, AppState};
use zk_srv_http::{connect, router_from, state};

const DEFAULT_ADDR: &str = "127.0.0.1:8787";
/// Session-level advisory lock key ("ZKSERV" slot); one lock per database.
const INSTANCE_LOCK_KEY: i64 = 0x5A4B_5345_5256_4552;
const SHUTDOWN_DEADLINE_SECS: u64 = 30;

#[tokio::main]
async fn main() -> Result<(), String> {
    let database_url = std::env::var("ZEOKEEL_DATABASE_URL")
        .map_err(|_| "ZEOKEEL_DATABASE_URL is required".to_owned())?;
    let pool = connect(&database_url)
        .await
        .map_err(|error| format!("connect PostgreSQL: {error}"))?;
    // Reject a second instance before migrations/jobs/grants can run twice.
    let _instance_lock = zk_kit_server::acquire_instance_lock(&pool, INSTANCE_LOCK_KEY)
        .await
        .map_err(|error| format!("acquire instance advisory lock: {error}"))?;
    let mut migrations: Vec<(&str, &str)> = zk_srv_http::MIGRATIONS
        .iter()
        .chain(zk_srv_auth_local::MIGRATIONS.iter())
        .chain(zk_srv_users::MIGRATIONS.iter())
        .chain(zk_srv_content::MIGRATIONS.iter())
        .chain(zk_srv_connectors::MIGRATIONS.iter())
        .chain(zk_srv_apps::MIGRATIONS.iter())
        .chain(zk_srv_wallet::MIGRATIONS.iter())
        .chain(zk_srv_pricing::MIGRATIONS.iter())
        .chain(zk_srv_gateway::MIGRATIONS.iter())
        .chain(zk_srv_audit::MIGRATIONS.iter())
        .chain(zk_srv_settings::MIGRATIONS.iter())
        .chain(zk_srv_jobs::MIGRATIONS.iter())
        .copied()
        .collect();
    // Domain manifests are chained in role order, not global version order.
    migrations.sort_by(|left, right| left.0.cmp(right.0));
    zk_kit_server::run_migrations(&pool, &migrations)
        .await
        .map_err(|error| format!("apply migrations: {error}"))?;
    let state = state(pool);
    // Startup compensation for admissions that crashed before settlement.
    let startup_key = format!("usage-pending:startup:{}", zk_kit_server::now_secs());
    if let Err(error) = zk_srv_jobs::run_job(&state.pool, "usage-pending", &startup_key).await {
        eprintln!("startup pending-usage compensation failed: {error}");
    }
    let jobs = zk_srv_jobs::spawn_loop(state.pool.clone());
    if let Ok(password) = std::env::var("ZEOKEEL_BOOTSTRAP_ADMIN_PASSWORD") {
        let username = std::env::var("ZEOKEEL_BOOTSTRAP_ADMIN_USERNAME")
            .unwrap_or_else(|_| "admin".to_owned());
        let created = ensure_bootstrap_admin(&state, &username, &password)
            .await
            .map_err(|error| format!("bootstrap admin: {error}"))?;
        if created {
            println!("bootstrap admin created: {username}");
        }
    }
    let app = router_from(state.clone())
        .merge(zk_srv_auth_local::routes().with_state(state.clone()))
        .merge(zk_srv_users::routes().with_state(state.clone()))
        .merge(zk_srv_content::routes().with_state(state.clone()))
        .merge(zk_srv_skills::routes().with_state(state.clone()))
        .merge(zk_srv_connectors::routes().with_state(state.clone()))
        .merge(zk_srv_apps::routes().with_state(state.clone()))
        .merge(zk_srv_updates::routes().with_state(state.clone()))
        .merge(zk_srv_wallet::routes().with_state(state.clone()))
        .merge(zk_srv_pricing::routes().with_state(state.clone()))
        .merge(zk_srv_gateway::routes().with_state(state.clone()))
        .merge(zk_srv_audit::routes().with_state(state.clone()))
        .merge(zk_srv_settings::routes().with_state(state.clone()))
        .merge(zk_srv_jobs::routes().with_state(state.clone()));
    let result = serve(app).await;
    // Stop cron and wait for the in-flight job tick to end (`docs/24` §6).
    jobs.abort();
    let _ = jobs.await;
    shutdown_sequence(&state).await;
    result
}

async fn serve(app: Router) -> Result<(), String> {
    let addr = std::env::var("ZEOKEEL_SERVER_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_owned());
    let admin_dist =
        std::env::var("ZEOKEEL_ADMIN_DIST").unwrap_or_else(|_| "apps/admin/web/dist".to_owned());
    let admin_index = format!("{admin_dist}/index.html");
    let app = app.nest_service(
        "/admin",
        ServeDir::new(&admin_dist).not_found_service(ServeFile::new(admin_index)),
    );
    // Native clients do not need CORS; allow an explicit dev origin for Playwright only.
    let app = if let Ok(origin) = std::env::var("ZEOKEEL_DEV_CORS_ORIGIN") {
        app.layer(
            CorsLayer::new()
                .allow_origin(
                    origin
                        .parse::<axum::http::HeaderValue>()
                        .map_err(|error| error.to_string())?,
                )
                .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                ]),
        )
    } else {
        app
    };
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|error| format!("bind {addr}: {error}"))?;
    println!("zk-server listening on http://{addr}");
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        let _ = shutdown_rx.await;
    });
    let server_task = tokio::spawn(server.into_future());
    shutdown_signal().await;
    let _ = shutdown_tx.send(());
    match tokio::time::timeout(Duration::from_secs(SHUTDOWN_DEADLINE_SECS), server_task).await {
        Ok(Ok(Ok(()))) => Ok(()),
        Ok(Ok(Err(error))) => Err(format!("zk-server failed: {error}")),
        Ok(Err(error)) => Err(format!("zk-server task failed: {error}")),
        Err(_) => Err(
            "shutdown deadline exceeded; residual pending usage left for startup compensation"
                .to_owned(),
        ),
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let terminate = async {
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(mut signal) => {
                    let _ = signal.recv().await;
                }
                Err(_) => std::future::pending::<()>().await,
            }
        };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            () = terminate => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

async fn shutdown_sequence(state: &AppState) {
    // Compensation + audit flush are bounded; a timeout leaves residuals for the
    // next startup sweep (`docs/24` §6 "超时 → 强制退出 1").
    let pending_key = format!("usage-pending:shutdown:{}", zk_kit_server::now_secs());
    match tokio::time::timeout(
        Duration::from_secs(SHUTDOWN_DEADLINE_SECS),
        zk_srv_jobs::run_job(&state.pool, "usage-pending", &pending_key),
    )
    .await
    {
        Ok(Ok(detail)) => println!("shutdown pending-usage sweep: {detail}"),
        Ok(Err(error)) => eprintln!("shutdown pending-usage sweep failed: {error}"),
        Err(_) => eprintln!("shutdown pending-usage sweep timed out"),
    }
    // Audit rows are written synchronously inside business transactions; the
    // final chain verification is the shutdown "flush" consistency check.
    let verify_key = format!("audit-verify:shutdown:{}", zk_kit_server::now_secs());
    match tokio::time::timeout(
        Duration::from_secs(SHUTDOWN_DEADLINE_SECS),
        zk_srv_jobs::run_job(&state.pool, "audit-verify", &verify_key),
    )
    .await
    {
        Ok(Ok(detail)) => println!("shutdown audit flush check: {detail}"),
        Ok(Err(error)) => eprintln!("shutdown audit flush check failed: {error}"),
        Err(_) => eprintln!("shutdown audit flush check timed out"),
    }
    state.pool.close().await;
    println!("zk-server shutdown complete");
}

#[cfg(test)]
mod tests {
    #[test]
    fn edition_is_composed() {
        assert!(!zk_composed::EDITION.is_empty());
    }
}
