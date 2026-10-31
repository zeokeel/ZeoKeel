#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only
fn write(name: &str, schema: &schemars::schema::RootSchema) {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dir = root.join("shared/contracts/schema");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(name),
        format!("{}\n", serde_json::to_string_pretty(schema).unwrap()),
    )
    .unwrap();
}

#[test]
fn export_schema() {
    write(
        "connector.json",
        &schemars::schema_for!(zk_contract_connector::Connector),
    );
    // OAuth public-client metadata (client-side PKCE flow, docs/07 §6) and the
    // catalog item that carries it. Generated only by this exporter (C11).
    write(
        "connector-oauth.json",
        &schemars::schema_for!(zk_contract_connector::ConnectorOAuth),
    );
    write(
        "connector-catalog-item.json",
        &schemars::schema_for!(zk_contract_connector::ConnectorCatalogItem),
    );
}
