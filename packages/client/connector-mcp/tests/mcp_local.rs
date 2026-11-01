#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test-only
//! Local-settings MCP source precedence + no-cache probe (spec R3/R5).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use zk_connector_mcp::{
    clear_catalog_config, clear_local_servers, configured_server_ids, session_count, test_server,
    update_from_catalog, update_local_servers, SERVERS_ENV,
};
use zk_contract_connector::{ConnectorTransport, McpServerConfig};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn script() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("fake_mcp_server.py")
        .to_string_lossy()
        .into_owned()
}

fn fixture(id: &str, mode: Option<&str>, countfile: &Path) -> McpServerConfig {
    let mut args = vec![script()];
    if let Some(mode) = mode {
        args.push(mode.to_owned());
    }
    if mode == Some("countfile") {
        args.push(countfile.to_string_lossy().into_owned());
    }
    McpServerConfig {
        id: id.to_owned(),
        name: id.to_owned(),
        transport: ConnectorTransport::Stdio,
        command: "python3".to_owned(),
        args,
        env: std::collections::BTreeMap::new(),
        url: String::new(),
        timeout_ms: 0,
        oauth_required: false,
    }
}

fn count(path: &Path) -> u64 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

#[test]
fn test_server_is_one_shot_and_does_not_touch_the_session_cache() {
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let countfile =
        std::env::temp_dir().join(format!("zk-mcp-probe-{}-{unique}.txt", std::process::id()));
    let probe = fixture("probe", Some("countfile"), &countfile);
    assert_eq!(session_count(), 0);
    let tools = test_server(&probe).unwrap();
    assert_eq!(tools, vec!["echo".to_owned()]);
    assert_eq!(count(&countfile), 1, "one initialize for the probe");
    assert_eq!(
        session_count(),
        0,
        "probe must not install a global session"
    );
    // A second probe is another fresh transport (still no cached session).
    let tools = test_server(&probe).unwrap();
    assert_eq!(tools, vec!["echo".to_owned()]);
    assert_eq!(count(&countfile), 2);
    assert_eq!(session_count(), 0);
    let invalid = McpServerConfig {
        command: String::new(),
        ..fixture("bad", None, &countfile)
    };
    assert!(test_server(&invalid).is_err());
    let _ = std::fs::remove_file(&countfile);
}

#[test]
fn source_precedence_catalog_even_empty_then_local_since_env() {
    // Isolate the process-wide slots for this test binary.
    clear_catalog_config().unwrap();
    clear_local_servers().unwrap();
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let countfile = std::env::temp_dir().join(format!(
        "zk-mcp-precedence-{}-{unique}.txt",
        std::process::id()
    ));
    let env_fixture = fixture("from-env", None, &countfile);
    std::env::set_var(
        SERVERS_ENV,
        serde_json::to_string(&vec![env_fixture]).unwrap(),
    );

    // Fresh install: env fallback applies.
    assert_eq!(
        configured_server_ids().unwrap(),
        vec!["from-env".to_owned()]
    );

    // Local settings (even empty) override env.
    update_local_servers(Vec::new()).unwrap();
    assert!(configured_server_ids().unwrap().is_empty());
    update_local_servers(vec![fixture("from-local", None, &countfile)]).unwrap();
    assert_eq!(
        configured_server_ids().unwrap(),
        vec!["from-local".to_owned()]
    );

    // Catalog (even empty) overrides local.
    update_from_catalog(r#"{"items":[]}"#).unwrap();
    assert!(configured_server_ids().unwrap().is_empty());

    // Clearing the catalog falls back to local, then clearing local to env.
    clear_catalog_config().unwrap();
    assert_eq!(
        configured_server_ids().unwrap(),
        vec!["from-local".to_owned()]
    );
    clear_local_servers().unwrap();
    assert_eq!(
        configured_server_ids().unwrap(),
        vec!["from-env".to_owned()]
    );
    std::env::remove_var(SERVERS_ENV);
    clear_catalog_config().unwrap();
    clear_local_servers().unwrap();
}
