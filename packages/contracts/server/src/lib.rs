//! Server-side domain contract marker (role `srv.http` and friends).
//!
//! @capability contract.server
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Stable server module descriptor exposed by `srv.*` packages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ServerModule {
    /// Namespaced module id (e.g. `srv/http`).
    pub id: String,
    /// Human-readable version.
    pub version: String,
}

impl ServerModule {
    /// Build a descriptor.
    #[must_use]
    pub fn new(id: &str, version: &str) -> Self {
        Self {
            id: id.to_owned(),
            version: version.to_owned(),
        }
    }
}

// ---------------------------------------------------------------------------
// Admin console HTTP wire responses (`/api/admin/*`).
//
// The admin SPA validates every response body against the generated JSON Schema
// (`shared/contracts/schema/admin-*-response.json`, `scripts/schema-ts.py`) before
// using it (C10/T22). These types mirror the exact wire shapes emitted by the
// `srv.*` admin routes; the server keeps emitting them from `serde_json::json!`
// (contract-first mirror, same pattern as `zk-contract-gateway::ModelPrice`).
// ---------------------------------------------------------------------------

/// Admin console login response (`POST /api/admin/auth/login`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminLoginResponse {
    /// Always `true` on success (errors use the HTTP error body).
    pub ok: bool,
    /// CSRF token required by subsequent admin writes.
    pub csrf_token: String,
    /// Admin session lifetime in seconds.
    pub expires_in: i64,
    /// Admin session idle timeout in seconds.
    pub idle_timeout_secs: i64,
    /// Admin role (`super_admin` or `auditor`).
    pub role: String,
}

/// Success envelope for admin mutations whose payload the SPA ignores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminOkResponse {
    /// Always `true` on success (errors use the HTTP error body).
    pub ok: bool,
}

/// One admin user row (`GET /api/admin/users`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminUserItem {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub status: String,
}

/// `GET /api/admin/users` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminUsersResponse {
    pub ok: bool,
    pub users: Vec<AdminUserItem>,
}

/// One admin wallet row (`GET /api/admin/wallet`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminWalletItem {
    pub user_id: String,
    pub username: String,
    pub balance_micros: i64,
}

/// `GET /api/admin/wallet` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminWalletResponse {
    pub ok: bool,
    pub items: Vec<AdminWalletItem>,
}

/// `GET /api/admin/pricing` response (rows match `ModelPrice`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminPriceListResponse {
    pub ok: bool,
    pub items: Vec<zk_contract_gateway::ModelPrice>,
}

/// One admin audit row (`GET /api/admin/audit`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminAuditItem {
    pub id: i64,
    pub actor_id: Option<String>,
    pub action: String,
    pub target: String,
    pub detail: serde_json::Value,
}

/// `GET /api/admin/audit` response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminAuditResponse {
    pub ok: bool,
    pub items: Vec<AdminAuditItem>,
}

/// One admin usage row (`GET /api/admin/usage`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminUsageItem {
    pub username: String,
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_micros: i64,
}

/// `GET /api/admin/usage` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminUsageResponse {
    pub ok: bool,
    pub items: Vec<AdminUsageItem>,
}

/// One job run row (`GET /api/admin/jobs`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminJobItem {
    pub job: String,
    pub idempotency_key: String,
    pub status: String,
    pub detail: serde_json::Value,
}

/// `GET /api/admin/jobs` response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminJobsResponse {
    pub ok: bool,
    pub items: Vec<AdminJobItem>,
}

/// One system-setting row (`GET /api/admin/settings`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminSettingItem {
    pub key: String,
    pub value: serde_json::Value,
    pub hot: bool,
}

/// `GET /api/admin/settings` response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminSettingsResponse {
    pub ok: bool,
    pub items: Vec<AdminSettingItem>,
}

/// One admin skill/content row (`GET /api/admin/skills`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminSkillItem {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub owner_id: String,
    pub status: String,
    pub visibility: String,
    pub current_release_id: Option<String>,
    pub force_update: bool,
    pub min_client_version: String,
    pub version: String,
}

/// `GET /api/admin/skills` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminSkillsResponse {
    pub ok: bool,
    pub items: Vec<AdminSkillItem>,
}

/// One admin connector row (`GET /api/admin/connectors`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminConnectorItem {
    pub id: String,
    pub name: String,
    pub transport: String,
    pub force_update: bool,
    pub min_client_version: String,
    pub grants: Vec<zk_contract_connector::ConnectorGrant>,
    pub oauth_client_id: Option<String>,
    pub redirect_uri: Option<String>,
}

/// `GET /api/admin/connectors` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminConnectorsResponse {
    pub ok: bool,
    pub items: Vec<AdminConnectorItem>,
}

/// One admin app row (`GET /api/admin/apps`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminAppItem {
    pub id: String,
    pub name: String,
    pub status: String,
    pub owner_id: String,
    pub version: String,
    pub wasm_sha256: String,
}

/// `GET /api/admin/apps` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AdminAppsResponse {
    pub ok: bool,
    pub items: Vec<AdminAppItem>,
}
