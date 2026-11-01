//! Local MCP settings application + two-phase probe broker (client-mcp-config spec R4/R5).
//!
//! Personal edition: `AppSettings.mcp.servers` is installed into the MCP client (source
//! priority: enterprise catalog > local settings > `ZEOKEEL_MCP_SERVERS` env fallback).
//! Enterprise edition: catalog-only — the local section is never installed, the env
//! fallback is disabled at boot, and `settings.set` rejects a non-empty local section.
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use zk_contract_config::AppSettings;
use zk_contract_connector::McpServerConfig;
use zk_contract_ipc::results::{McpTestState, McpTestStatusResult};

/// Hard probe deadline (spec R5: `mcp.test` is capped at 15s).
pub(crate) const MAX_PROBE_MS: u64 = 15_000;
/// Default probe deadline when the draft server does not set `timeout_ms`.
const DEFAULT_PROBE_MS: u64 = 10_000;

static PROBE_SEQ: AtomicU64 = AtomicU64::new(1);

type ProbeMap = Arc<Mutex<HashMap<String, McpTestStatusResult>>>;

/// One in-flight/finished probe registry (`pending` while the worker runs).
pub(crate) struct McpProbeBroker {
    probes: ProbeMap,
}

impl McpProbeBroker {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            probes: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Start a no-cache probe for one draft server; returns the `test_id` to poll.
    ///
    /// # Errors
    ///
    /// `mcp/test-in-flight` while another probe is still `pending`.
    pub(crate) fn start(&self, server: McpServerConfig) -> Result<String, String> {
        let test_id = format!("mcp-test-{}", PROBE_SEQ.fetch_add(1, Ordering::Relaxed));
        {
            let mut probes = self
                .probes
                .lock()
                .map_err(|_| "mcp/unsupported: probe registry poisoned".to_owned())?;
            if probes
                .values()
                .any(|status| status.state == McpTestState::Pending)
            {
                return Err("mcp/test-in-flight".to_owned());
            }
            probes.insert(
                test_id.clone(),
                McpTestStatusResult {
                    state: McpTestState::Pending,
                    tools: Vec::new(),
                    error: None,
                },
            );
        }
        let probes = Arc::clone(&self.probes);
        let id = test_id.clone();
        std::thread::spawn(move || {
            let result = run_probe(server);
            if let Ok(mut map) = probes.lock() {
                map.insert(id, result);
            }
        });
        Ok(test_id)
    }

    /// Read one probe; `None` for an unknown/expired `test_id`.
    pub(crate) fn status(&self, test_id: &str) -> Option<McpTestStatusResult> {
        self.probes.lock().ok()?.get(test_id).cloned()
    }
}

/// Run one probe with a hard deadline: the worker thread may outlive the deadline
/// (bounded by the transport's own timeout), but the status becomes `mcp/timeout`.
fn run_probe(server: McpServerConfig) -> McpTestStatusResult {
    let (tx, rx) = mpsc::channel::<McpTestStatusResult>();
    std::thread::spawn(move || {
        let _ = tx.send(probe_once(server));
    });
    rx.recv_timeout(Duration::from_millis(MAX_PROBE_MS))
        .unwrap_or(McpTestStatusResult {
            state: McpTestState::Error,
            tools: Vec::new(),
            error: Some("mcp/timeout".to_owned()),
        })
}

fn probe_once(mut server: McpServerConfig) -> McpTestStatusResult {
    let timeout = if server.timeout_ms == 0 {
        DEFAULT_PROBE_MS
    } else {
        server.timeout_ms.min(MAX_PROBE_MS)
    };
    server.timeout_ms = timeout;
    match zk_connector_mcp::test_server(&server) {
        Ok(tools) => McpTestStatusResult {
            state: McpTestState::Ok,
            tools,
            error: None,
        },
        Err(error) => McpTestStatusResult {
            state: McpTestState::Error,
            tools: Vec::new(),
            error: Some(probe_error_code(&error)),
        },
    }
}

fn probe_error_code(error: &str) -> String {
    if error.starts_with("MCP_VALIDATION") {
        return "mcp/validation".to_owned();
    }
    let lowered = error.to_ascii_lowercase();
    if lowered.contains("timed out") || lowered.contains("timeout") {
        return "mcp/timeout".to_owned();
    }
    "mcp/connect-failed".to_owned()
}

/// Install local MCP settings into the live client (personal) / reject (enterprise).
///
/// # Errors
///
/// Validation/install errors or the enterprise catalog-only rejection.
pub(crate) fn apply_settings_mcp(settings: &AppSettings) -> Result<(), String> {
    if zk_composed::EDITION == "personal" {
        zk_connector_mcp::update_local_servers(settings.mcp.servers.clone())
            .map_err(|error| format!("mcp/validation: {error}"))
    } else if settings.mcp.servers.is_empty() {
        Ok(())
    } else {
        Err(
            "mcp/edition-unsupported: enterprise is catalog-only; local MCP servers are not available"
                .to_owned(),
        )
    }
}

/// Boot application: install the persisted local section only when a settings file
/// exists (fresh install keeps the env fallback); enterprise clears everything.
///
/// # Errors
///
/// Propagates validation/state errors so boot can log them honestly.
pub(crate) fn apply_persisted_mcp(settings: &AppSettings, file_exists: bool) -> Result<(), String> {
    if zk_composed::EDITION == "personal" {
        if file_exists {
            zk_connector_mcp::update_local_servers(settings.mcp.servers.clone())
                .map_err(|error| format!("mcp/validation: {error}"))
        } else {
            zk_connector_mcp::clear_local_servers()
        }
    } else {
        // Catalog-only: ignore/clear the local section and disable the env fallback.
        zk_connector_mcp::clear_local_servers()?;
        zk_connector_mcp::set_env_fallback_enabled(false)
    }
}
