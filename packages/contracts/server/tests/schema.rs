#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
use std::path::PathBuf;

fn write(name: &str, schema: &schemars::schema::RootSchema) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = root.join("shared/contracts/schema");
    std::fs::create_dir_all(&dir).unwrap();
    let json = serde_json::to_string_pretty(schema).unwrap();
    std::fs::write(dir.join(name), format!("{json}\n")).unwrap();
}

#[test]
fn export_schema() {
    write(
        "server-module.json",
        &schemars::schema_for!(zk_contract_server::ServerModule),
    );
}

#[test]
fn export_admin_api_schema() {
    // Admin SPA response boundary (C10/T22): every `/api/admin/*` body the SPA
    // parses is validated against these generated schemas (`admin-ui-kit`).
    for (name, schema) in [
        (
            "admin-login-response.json",
            schemars::schema_for!(zk_contract_server::AdminLoginResponse),
        ),
        (
            "admin-ok-response.json",
            schemars::schema_for!(zk_contract_server::AdminOkResponse),
        ),
        (
            "admin-users-response.json",
            schemars::schema_for!(zk_contract_server::AdminUsersResponse),
        ),
        (
            "admin-wallet-response.json",
            schemars::schema_for!(zk_contract_server::AdminWalletResponse),
        ),
        (
            "admin-price-list-response.json",
            schemars::schema_for!(zk_contract_server::AdminPriceListResponse),
        ),
        (
            "admin-audit-response.json",
            schemars::schema_for!(zk_contract_server::AdminAuditResponse),
        ),
        (
            "admin-usage-response.json",
            schemars::schema_for!(zk_contract_server::AdminUsageResponse),
        ),
        (
            "admin-jobs-response.json",
            schemars::schema_for!(zk_contract_server::AdminJobsResponse),
        ),
        (
            "admin-settings-response.json",
            schemars::schema_for!(zk_contract_server::AdminSettingsResponse),
        ),
        (
            "admin-skills-response.json",
            schemars::schema_for!(zk_contract_server::AdminSkillsResponse),
        ),
        (
            "admin-connectors-response.json",
            schemars::schema_for!(zk_contract_server::AdminConnectorsResponse),
        ),
        (
            "admin-apps-response.json",
            schemars::schema_for!(zk_contract_server::AdminAppsResponse),
        ),
    ] {
        write(name, &schema);
    }
}
