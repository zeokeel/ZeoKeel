//! `zk-server`: enterprise thin-management companion (M2 P1).
//!
//! The server never runs an agent or skill. P1 exposes health/readiness and applies
//! the immutable schema; auth/admin/wallet/gateway land in subsequent P1–P2 slices.
#![forbid(unsafe_code)]

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use zk_kit_server::ensure_bootstrap_admin;
use zk_srv_http::{connect, router_from, state};

const DEFAULT_ADDR: &str = "127.0.0.1:8787";

#[tokio::main]
async fn main() -> Result<(), String> {
    let database_url = std::env::var("ZEOKEEL_DATABASE_URL")
        .map_err(|_| "ZEOKEEL_DATABASE_URL is required".to_owned())?;
    let pool = connect(&database_url)
        .await
        .map_err(|error| format!("connect PostgreSQL: {error}"))?;
    let migrations: Vec<(&str, &str)> = zk_srv_http::MIGRATIONS
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
    zk_kit_server::run_migrations(&pool, &migrations)
        .await
        .map_err(|error| format!("apply migrations: {error}"))?;
    let state = state(pool);
    zk_srv_jobs::spawn_loop(state.pool.clone());
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
        .merge(zk_srv_jobs::routes().with_state(state));
    serve(app).await
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
