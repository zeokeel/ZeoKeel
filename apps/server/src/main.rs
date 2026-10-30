//! `zk-server`: enterprise thin-management companion (M2 P1).
//!
//! The server never runs an agent or skill. It applies the immutable schema
//! (fail-closed checksum verification), takes a single-instance PostgreSQL
//! advisory lock, and on SIGTERM drains requests, compensates pending usage and
//! verifies the audit chain before closing the pool (`docs/24` §5–§6).
#![forbid(unsafe_code)]

use std::future::IntoFuture;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use zk_kit_server::{ensure_bootstrap_admin, AppState};
use zk_srv_http::{connect, router_from, state};

const DEFAULT_ADDR: &str = "127.0.0.1:8787";
/// Session-level advisory lock key ("ZKSERV" slot); one lock per database.
const INSTANCE_LOCK_KEY: i64 = 0x5A4B_5345_5256_4552;
const SHUTDOWN_DEADLINE_SECS: u64 = 30;
const MIN_SHUTDOWN_BUDGET_SECS: u64 = 5;
const MAX_SHUTDOWN_BUDGET_SECS: u64 = 120;

/// One total SIGTERM budget: drain, jobs stop, settlement, audit, pool close and
/// kernel disposal all share it; expiration forces exit 1 (`docs/24` §6).
struct ShutdownBudget {
    deadline: Instant,
}

impl ShutdownBudget {
    fn new() -> Self {
        let now = Instant::now();
        let deadline = now.checked_add(shutdown_budget()).unwrap_or(now);
        Self { deadline }
    }

    fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }

    fn expired(&self) -> bool {
        self.remaining().is_zero()
    }
}

fn shutdown_budget() -> Duration {
    let secs = std::env::var("ZEOKEEL_SHUTDOWN_DEADLINE_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(SHUTDOWN_DEADLINE_SECS)
        .clamp(MIN_SHUTDOWN_BUDGET_SECS, MAX_SHUTDOWN_BUDGET_SECS);
    Duration::from_secs(secs)
}

fn force_exit(reason: &str) -> ! {
    eprintln!("zk-server: {reason}; forcing exit 1");
    std::process::exit(1);
}

struct ServeOutcome {
    budget: ShutdownBudget,
    result: Result<(), String>,
}

/// Kernel service: server state (PG pool + JWT keys).
static SERVER_STATE_SERVICE: zk_bootstrap::ServiceKey<AppState> =
    zk_bootstrap::ServiceKey::new("zk.server.state");
/// Kernel service: server identity (version/edition).
static SERVER_RUNTIME_SERVICE: zk_bootstrap::ServiceKey<ServerRuntimeIdentity> =
    zk_bootstrap::ServiceKey::new("zk.server.runtime");

/// Identity of the server runtime, exposed as a kernel service.
#[derive(Clone, Debug)]
struct ServerRuntimeIdentity {
    version: &'static str,
    edition: &'static str,
}

/// Plugin that registers the server identity in the kernel (ADR-009).
struct ServerRuntimePlugin {
    identity: ServerRuntimeIdentity,
}

impl zk_bootstrap::Plugin for ServerRuntimePlugin {
    fn name(&self) -> &'static str {
        "zk.server.runtime"
    }

    fn apply(
        &self,
        ctx: zk_bootstrap::Context,
        _config: serde_json::Value,
    ) -> zk_bootstrap::CordisResult<()> {
        ctx.provide(SERVER_RUNTIME_SERVICE, Arc::new(self.identity.clone()))?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    // Kernel root before any pool/route/task work (docs/06 §1).
    let bootstrap =
        zk_bootstrap::boot("zk-server").map_err(|error| format!("boot kernel: {error}"))?;
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
    bootstrap
        .provide(SERVER_STATE_SERVICE, Arc::new(state.clone()))
        .map_err(|error| format!("register server state: {error}"))?;
    let identity = ServerRuntimeIdentity {
        version: env!("CARGO_PKG_VERSION"),
        edition: zk_composed::EDITION,
    };
    println!(
        "zk-server: composed runtime edition={} version={}",
        identity.edition, identity.version
    );
    bootstrap
        .register_plugin(
            Arc::new(ServerRuntimePlugin { identity }),
            serde_json::json!({ "edition": zk_composed::EDITION }),
        )
        .map_err(|error| format!("register server runtime: {error}"))?;
    // Startup compensation for admissions that crashed before settlement.
    let startup_key = format!("usage-pending:startup:{}", zk_kit_server::now_secs());
    if let Err(error) = zk_srv_jobs::run_job(&state.pool, "usage-pending", &startup_key).await {
        eprintln!("startup pending-usage compensation failed: {error}");
    }
    // The job loop runs under kernel supervision; the package's stop token is
    // signalled after the HTTP drain so the in-flight tick can finish before the
    // pool is closed.
    let (jobs_tx, jobs_rx) = tokio::sync::oneshot::channel::<zk_srv_jobs::JobsLoop>();
    let jobs_pool = state.pool.clone();
    let _jobs_supervisor = bootstrap
        .spawn_task("srv.jobs", async move {
            let jobs = zk_srv_jobs::spawn_loop(jobs_pool);
            let _ = jobs_tx.send(jobs);
            std::future::pending::<()>().await;
        })
        .map_err(|error| format!("start jobs scheduler: {error}"))?;
    let jobs = jobs_rx
        .await
        .map_err(|_| "jobs scheduler stopped before its loop was registered".to_owned())?;
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
        .merge(zk_srv_jobs::routes().with_state(state.clone()))
        .layer(axum::middleware::from_fn(zk_srv_http::client_version_gate));
    let outcome = serve(app)
        .await
        .map_err(|error| format!("zk-server: {error}"))?;
    let budget = outcome.budget;
    let mut final_error = outcome.result.err();
    if budget.expired() {
        force_exit("drain exceeded shutdown budget");
    }
    // Stop cron and wait (bounded by the total shutdown budget) for the in-flight
    // job tick before settlement/audit closes the pool (`docs/24` §6).
    jobs.stop();
    match tokio::time::timeout(budget.remaining(), jobs.wait()).await {
        Ok(true) => {}
        Ok(false) => {
            eprintln!("zk-server: jobs loop exceeded the bounded wait; aborted before settlement");
            if final_error.is_none() {
                final_error = Some("jobs loop exceeded the bounded wait".to_owned());
            }
        }
        Err(_) => force_exit("jobs stop exceeded shutdown budget"),
    }
    if budget.expired() {
        force_exit("jobs stop exceeded shutdown budget");
    }
    shutdown_sequence(&state, &budget).await;
    let remaining = budget.remaining();
    if remaining.is_zero() {
        force_exit("settlement/audit exceeded shutdown budget");
    }
    // Kernel disposal is synchronous and has no internal deadline; bound it with
    // the remaining total budget and force-exit if it cannot finish in time.
    let bootstrap = Arc::new(bootstrap);
    match tokio::time::timeout(
        remaining,
        tokio::task::spawn_blocking(move || bootstrap.shutdown()),
    )
    .await
    {
        Ok(Ok(Ok(()))) => {}
        Ok(Ok(Err(error))) => {
            eprintln!("zk-server: bootstrap shutdown failed: {error}");
            if final_error.is_none() {
                final_error = Some(format!("bootstrap shutdown failed: {error}"));
            }
        }
        Ok(Err(error)) => {
            eprintln!("zk-server: bootstrap shutdown task failed: {error}");
            if final_error.is_none() {
                final_error = Some(format!("bootstrap shutdown task failed: {error}"));
            }
        }
        Err(_) => force_exit("bootstrap shutdown exceeded shutdown budget"),
    }
    if let Some(error) = final_error {
        return Err(error);
    }
    Ok(())
}

async fn serve(app: Router) -> Result<ServeOutcome, String> {
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
                    axum::http::header::HeaderName::from_static("x-zeokeel-client"),
                    axum::http::header::HeaderName::from_static("x-request-id"),
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
    // Start the single total budget when SIGTERM arrives: every later phase uses
    // its remaining time and the process force-exits 1 when it is exhausted.
    let budget = ShutdownBudget::new();
    let _ = shutdown_tx.send(());
    let result = match tokio::time::timeout(budget.remaining(), server_task).await {
        Ok(Ok(Ok(()))) => Ok(()),
        Ok(Ok(Err(error))) => Err(format!("zk-server failed: {error}")),
        Ok(Err(error)) => Err(format!("zk-server task failed: {error}")),
        Err(_) => Err(
            "drain exceeded shutdown budget; residual pending usage left for startup compensation"
                .to_owned(),
        ),
    };
    Ok(ServeOutcome { budget, result })
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

async fn shutdown_sequence(state: &AppState, budget: &ShutdownBudget) {
    // Compensation + audit flush are bounded by the remaining total budget; a
    // timeout leaves residuals for the next startup sweep (`docs/24` §6
    // "超时 → 强制退出 1").
    let pending_key = format!("usage-pending:shutdown:{}", zk_kit_server::now_secs());
    let remaining = budget.remaining();
    if remaining.is_zero() {
        force_exit("pending settlement exceeded shutdown budget");
    }
    match tokio::time::timeout(
        remaining,
        zk_srv_jobs::run_job(&state.pool, "usage-pending", &pending_key),
    )
    .await
    {
        Ok(Ok(detail)) => println!("shutdown pending-usage sweep: {detail}"),
        Ok(Err(error)) => eprintln!("shutdown pending-usage sweep failed: {error}"),
        Err(_) => force_exit("pending settlement exceeded shutdown budget"),
    }
    if budget.expired() {
        force_exit("pending settlement exceeded shutdown budget");
    }
    // Audit rows are written synchronously inside business transactions; the
    // final chain verification is the shutdown "flush" consistency check.
    let verify_key = format!("audit-verify:shutdown:{}", zk_kit_server::now_secs());
    let remaining = budget.remaining();
    if remaining.is_zero() {
        force_exit("audit flush exceeded shutdown budget");
    }
    match tokio::time::timeout(
        remaining,
        zk_srv_jobs::run_job(&state.pool, "audit-verify", &verify_key),
    )
    .await
    {
        Ok(Ok(detail)) => println!("shutdown audit flush check: {detail}"),
        Ok(Err(error)) => eprintln!("shutdown audit flush check failed: {error}"),
        Err(_) => force_exit("audit flush exceeded shutdown budget"),
    }
    if budget.expired() {
        force_exit("audit flush exceeded shutdown budget");
    }
    let remaining = budget.remaining();
    if remaining.is_zero() {
        force_exit("pool close exceeded shutdown budget");
    }
    match tokio::time::timeout(remaining, state.pool.close()).await {
        Ok(()) => {}
        Err(_) => force_exit("pool close exceeded shutdown budget"),
    }
    println!("zk-server shutdown complete");
}

#[cfg(test)]
mod tests {
    #[test]
    fn edition_is_composed() {
        assert!(!zk_composed::EDITION.is_empty());
    }

    #[test]
    fn shutdown_budget_is_bounded() {
        let budget = super::ShutdownBudget::new();
        assert!(!budget.expired());
        assert!(budget.remaining() <= super::shutdown_budget());
        assert!(super::shutdown_budget() >= std::time::Duration::from_secs(5));
    }
}
