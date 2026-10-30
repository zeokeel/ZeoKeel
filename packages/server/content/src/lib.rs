//! Content aggregate core (`srv.content`): status/releases/grants/deployments.
//!
//! M3 slice: schema + service seam; handlers land in subsequent tasks.
#![forbid(unsafe_code)]

use axum::Router;
use zk_kit_server::AppState;

/// Immutable content migrations owned by this package.
pub const MIGRATIONS: &[(&str, &str)] = &[("0010_content", include_str!("../migrations/0010_content.sql"))];

/// Content routes (M3 slice 3).
pub fn routes() -> Router<AppState> {
    Router::new()
}
