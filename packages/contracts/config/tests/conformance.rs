//! Contract conformance: settings round-trip and defaults.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)] // test-only

use zk_contract_config::{
    AgentSettings, AppSettings, ApprovalPolicy, EmbeddingSettings, MediaSettings, ProviderKind,
    RuntimeSettings, ScheduleSettings, ToolChoiceSetting, ToolSettings, WebSearchSettings,
};

#[test]
fn settings_round_trip_and_defaults() {
    let s = AppSettings::default();
    assert_eq!(s.provider.kind, ProviderKind::Mock);
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(s, serde_json::from_str::<AppSettings>(&json).unwrap());
}

#[test]
fn legacy_settings_without_agent_still_parse() {
    // Backward compatible: schema stays 1 and `agent` is serde-defaulted.
    let legacy = r#"{
        "schema": 1,
        "provider": {"kind": "mock", "base_url": "", "model": "mock", "api_key": ""},
        "theme": "auto",
        "locale": "zh-CN",
        "auto_approve_read_only": true,
        "server_url": ""
    }"#;
    let settings: AppSettings = serde_json::from_str(legacy).unwrap();
    assert_eq!(settings.agent, AgentSettings::default());
    assert_eq!(settings.agent.max_steps, 50);
    assert!(settings.agent.completion.stop.is_empty());
    assert_eq!(
        settings.agent.completion.tool_choice,
        ToolChoiceSetting::Auto
    );
    assert!(settings.agent.memory.enabled);
    assert!(settings.agent.hooks.enabled);
    assert_eq!(settings.tools, ToolSettings::default());
    assert_eq!(settings.agent.schedule, ScheduleSettings::default());
    assert_eq!(settings.embedding, EmbeddingSettings::default());
    assert!(!settings.embedding.is_configured());
    settings.embedding.validate().unwrap();
    assert_eq!(settings.media, MediaSettings::default());
    assert!(!settings.media.is_configured());
    settings.media.validate().unwrap();
    assert!(!settings.agent.schedule.enabled);
    assert_eq!(settings.agent.schedule.scan_seconds, 10);
    settings.agent.schedule.validate().unwrap();
    assert_eq!(settings.tools.tool_timeout_ms, 120_000);
    assert_eq!(settings.tools.approval_timeout_ms, 300_000);
    settings.tools.validate().unwrap();
    assert_eq!(settings.web_search, WebSearchSettings::default());
    assert_eq!(settings.web_search.timeout_ms, 15_000);
    settings.web_search.validate().unwrap();
}

#[test]
fn tool_settings_validate_and_clamp() {
    let settings = ToolSettings::default();
    settings.validate().unwrap();
    assert_eq!(settings.tool_timeout(), 120_000);
    assert_eq!(settings.approval_timeout(), 300_000);

    let mut bad = settings;
    bad.tool_timeout_ms = 0;
    assert!(bad.validate().is_err());
    assert_eq!(bad.tool_timeout(), ToolSettings::MIN_TOOL_TIMEOUT_MS);
    let mut bad = settings;
    bad.tool_timeout_ms = 900_000;
    assert!(bad.validate().is_err());
    assert_eq!(bad.tool_timeout(), ToolSettings::MAX_TOOL_TIMEOUT_MS);
    let mut bad = settings;
    bad.approval_timeout_ms = 0;
    assert!(bad.validate().is_err());
    assert_eq!(
        bad.approval_timeout(),
        ToolSettings::MIN_APPROVAL_TIMEOUT_MS
    );
    let mut bad = settings;
    bad.approval_timeout_ms = 2_000_000;
    assert!(bad.validate().is_err());
    assert_eq!(
        bad.approval_timeout(),
        ToolSettings::MAX_APPROVAL_TIMEOUT_MS
    );

    // Round-trips through serde.
    let json = serde_json::to_string(&settings).unwrap();
    assert_eq!(
        settings,
        serde_json::from_str::<ToolSettings>(&json).unwrap()
    );
}

#[test]
fn agent_settings_defaults_validate_and_round_trip() {
    let settings = AgentSettings::default();
    settings.validate().unwrap();
    let json = serde_json::to_string(&settings).unwrap();
    assert_eq!(
        settings,
        serde_json::from_str::<AgentSettings>(&json).unwrap()
    );

    let mut bad = settings.clone();
    bad.completion.temperature = Some(3.0);
    assert!(bad.validate().is_err());
    let mut bad = settings.clone();
    bad.completion.top_p = Some(1.5);
    assert!(bad.validate().is_err());
    let mut bad = settings.clone();
    bad.completion.max_tokens = Some(0);
    assert!(bad.validate().is_err());
    let mut bad = settings.clone();
    bad.max_steps = 0;
    assert!(bad.validate().is_err());
    let mut bad = settings.clone();
    bad.completion.tool_choice = ToolChoiceSetting::Specific(Vec::new());
    assert!(bad.validate().is_err());
    let mut bad = settings.clone();
    bad.completion.reasoning_effort = Some("extreme".to_owned());
    assert!(bad.validate().is_err());
    let mut bad = settings.clone();
    bad.completion.prompt_cache = Some("forever".to_owned());
    assert!(bad.validate().is_err());
}

#[test]
fn runtime_settings_snapshot_excludes_provider_secrets() {
    let mut settings = AppSettings::default();
    settings.provider.api_key = "sk-live-super-secret".to_owned();
    settings.provider.base_url = "https://upstream.example".to_owned();
    settings.agent.completion.temperature = Some(0.3);
    settings.tools.tool_timeout_ms = 2_000;
    settings.web_search = WebSearchSettings {
        base_url: "https://search.example.invalid/".to_owned(),
        timeout_ms: 20_000,
    };

    let snapshot = RuntimeSettings {
        agent: Some(settings.agent.clone()),
        tools: Some(settings.tools),
        web_search: Some(settings.web_search.clone()),
    };
    snapshot.validate().unwrap();
    zk_contract_config::set_runtime_settings(&snapshot).unwrap();
    let published = zk_contract_config::runtime_settings().unwrap().unwrap();
    assert_eq!(
        published.agent.as_ref().unwrap().completion.temperature,
        Some(0.3)
    );
    assert_eq!(published.tools.unwrap().tool_timeout_ms, 2_000);
    let web_search = published.web_search.clone().unwrap();
    assert_eq!(web_search.base_url, "https://search.example.invalid/");
    assert_eq!(web_search.timeout_ms, 20_000);

    // The whole published snapshot must be non-secret: no provider config at
    // all, hence no plaintext api_key/base_url.
    let json = serde_json::to_string(&published).unwrap();
    assert!(
        !json.contains("api_key"),
        "snapshot leaked api_key field: {json}"
    );
    assert!(
        !json.contains("sk-live-super-secret"),
        "snapshot leaked api_key value: {json}"
    );
    assert!(
        !json.contains("upstream.example"),
        "snapshot leaked provider base_url: {json}"
    );
    assert!(
        !json.contains("provider"),
        "snapshot leaked provider section: {json}"
    );

    zk_contract_config::clear_runtime_settings().unwrap();
}

#[test]
fn runtime_settings_sections_are_optional_and_legacy_snapshots_parse() {
    // Per-section publishing: tools-only saves must not publish agent defaults.
    let tools_only = RuntimeSettings {
        agent: None,
        tools: Some(ToolSettings {
            tool_timeout_ms: 2_000,
            ..ToolSettings::default()
        }),
        web_search: None,
    };
    tools_only.validate().unwrap();
    let json = serde_json::to_string(&tools_only).unwrap();
    assert!(!json.contains("\"agent\""), "None section omitted: {json}");
    let parsed: RuntimeSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, tools_only);

    let agent_only = RuntimeSettings {
        agent: Some(zk_contract_config::AgentSettings {
            max_steps: 12,
            ..zk_contract_config::AgentSettings::default()
        }),
        tools: None,
        web_search: None,
    };
    agent_only.validate().unwrap();
    assert!(RuntimeSettings::default().validate().is_ok());

    // Old full snapshots (both objects, no Options on the wire) still parse.
    let legacy = serde_json::json!({
        "agent": {"max_steps": 12},
        "tools": {"tool_timeout_ms": 2000, "approval_timeout_ms": 300000}
    });
    let restored: RuntimeSettings = serde_json::from_value(legacy).unwrap();
    assert_eq!(restored.agent.as_ref().unwrap().max_steps, 12);
    assert_eq!(restored.tools.unwrap().tool_timeout_ms, 2_000);

    // Validation still rejects bad values inside a present section.
    let bad = RuntimeSettings {
        tools: Some(ToolSettings {
            tool_timeout_ms: 10,
            ..ToolSettings::default()
        }),
        ..RuntimeSettings::default()
    };
    assert!(bad.validate().is_err());
}

#[test]
fn schedule_settings_validate_and_clamp() {
    let defaults = ScheduleSettings::default();
    assert!(!defaults.enabled);
    assert_eq!(defaults.scan_seconds, 10);
    assert_eq!(defaults.effective_scan_seconds(), 10);
    defaults.validate().unwrap();

    let mut bad = defaults;
    bad.scan_seconds = 4;
    assert!(bad.validate().is_err());
    assert_eq!(
        bad.effective_scan_seconds(),
        ScheduleSettings::MIN_SCAN_SECONDS
    );
    let mut bad = defaults;
    bad.scan_seconds = 3_601;
    assert!(bad.validate().is_err());
    assert_eq!(
        bad.effective_scan_seconds(),
        ScheduleSettings::MAX_SCAN_SECONDS
    );
    for seconds in [5, 10, 3_600] {
        let settings = ScheduleSettings {
            enabled: true,
            scan_seconds: seconds,
        };
        settings.validate().unwrap();
        assert_eq!(
            settings,
            serde_json::from_str::<ScheduleSettings>(&serde_json::to_string(&settings).unwrap())
                .unwrap()
        );
    }

    // AgentSettings::validate delegates to schedule validation.
    let mut invalid_agent = AgentSettings::default();
    invalid_agent.schedule.scan_seconds = 4;
    assert!(invalid_agent.validate().is_err());
    invalid_agent.schedule.scan_seconds = 3_601;
    assert!(invalid_agent.validate().is_err());
    invalid_agent.schedule.scan_seconds = 3_600;
    invalid_agent.validate().unwrap();

    // Legacy agent settings without `schedule` parse with defaults.
    let legacy: AgentSettings = serde_json::from_str(
        r#"{"max_steps":50,"completion":{},"memory":{"enabled":true,"dir":""},"hooks":{"enabled":true,"log":""}}"#,
    )
    .unwrap();
    assert_eq!(legacy.schedule, ScheduleSettings::default());
}

#[test]
fn embedding_settings_defaults_validation_and_legacy_parse() {
    let defaults = EmbeddingSettings::default();
    assert_eq!(defaults.base_url, "");
    assert_eq!(defaults.api_key, "");
    assert_eq!(defaults.model, "");
    assert!(defaults.dims.is_none());
    assert_eq!(defaults.dir, "");
    assert!(!defaults.is_configured());
    defaults.validate().unwrap();

    let configured = EmbeddingSettings {
        base_url: "https://api.example.invalid/v1".to_owned(),
        api_key: "sk-test".to_owned(),
        model: "text-embedding-3-small".to_owned(),
        dims: Some(1_536),
        dir: "/tmp/vector-index".to_owned(),
    };
    assert!(configured.is_configured());
    configured.validate().unwrap();
    assert_eq!(
        configured,
        serde_json::from_str::<EmbeddingSettings>(&serde_json::to_string(&configured).unwrap())
            .unwrap()
    );
    for dims in [0, 65_537] {
        let invalid = EmbeddingSettings {
            dims: Some(dims),
            ..configured.clone()
        };
        assert!(invalid.validate().is_err(), "dims {dims} must be rejected");
    }
    for base_url in ["ftp://example.invalid", "api.example.invalid"] {
        let invalid = EmbeddingSettings {
            base_url: base_url.to_owned(),
            ..configured.clone()
        };
        assert!(
            invalid.validate().is_err(),
            "scheme {base_url} must be rejected"
        );
    }
    let configured = EmbeddingSettings {
        base_url: "https://api.example.invalid/v1".to_owned(),
        api_key: "sk-super-secret".to_owned(),
        model: "text-embedding-3-small".to_owned(),
        dims: Some(8),
        dir: String::new(),
    };
    let debug = format!("{configured:?}");
    assert!(
        !debug.contains("sk-super-secret"),
        "Debug leaked key: {debug}"
    );
    assert!(debug.contains("[redacted]"), "{debug}");

    let invalid_app = AppSettings {
        embedding: EmbeddingSettings {
            dims: Some(0),
            ..EmbeddingSettings::default()
        },
        ..AppSettings::default()
    };
    assert!(invalid_app.validate().is_err());

    // Legacy settings JSON without `embedding` keeps backwards compatibility.
    let legacy: AppSettings = serde_json::from_str(
        r#"{"schema":1,"provider":{"kind":"mock"},"theme":"auto","locale":"zh-CN","auto_approve_read_only":true}"#,
    )
    .unwrap();
    assert_eq!(legacy.embedding, EmbeddingSettings::default());
}

#[test]
fn media_settings_defaults_validation_debug_and_legacy_parse() {
    let defaults = MediaSettings::default();
    assert_eq!(defaults.base_url, "");
    assert_eq!(defaults.api_key, "");
    assert_eq!(defaults.image_model, "");
    assert_eq!(defaults.audio_model, "");
    assert_eq!(defaults.transcription_model, "");
    assert_eq!(defaults.rerank_model, "");
    assert!(!defaults.is_configured());
    defaults.validate().unwrap();

    let configured = MediaSettings {
        base_url: "https://api.example.invalid/v1".to_owned(),
        api_key: "sk-media-secret".to_owned(),
        image_model: "gpt-image-1".to_owned(),
        audio_model: "tts-1".to_owned(),
        transcription_model: "whisper-1".to_owned(),
        rerank_model: "rerank-2.5".to_owned(),
    };
    assert!(configured.is_configured());
    configured.validate().unwrap();
    assert_eq!(
        configured,
        serde_json::from_str::<MediaSettings>(&serde_json::to_string(&configured).unwrap())
            .unwrap()
    );
    let debug = format!("{configured:?}");
    assert!(
        !debug.contains("sk-media-secret"),
        "Debug leaked key: {debug}"
    );
    assert!(debug.contains("[redacted]"), "{debug}");

    for base_url in ["ftp://example.invalid", "api.example.invalid"] {
        let invalid = MediaSettings {
            base_url: base_url.to_owned(),
            ..configured.clone()
        };
        assert!(
            invalid.validate().is_err(),
            "scheme {base_url} must be rejected"
        );
    }
    let invalid_app = AppSettings {
        media: MediaSettings {
            base_url: "ftp://example.invalid".to_owned(),
            ..MediaSettings::default()
        },
        ..AppSettings::default()
    };
    assert!(invalid_app.validate().is_err());

    let legacy: AppSettings = serde_json::from_str(
        r#"{"schema":1,"provider":{"kind":"mock"},"theme":"auto","locale":"zh-CN","auto_approve_read_only":true}"#,
    )
    .unwrap();
    assert_eq!(legacy.media, MediaSettings::default());
}

#[test]
fn web_search_settings_defaults_validation_and_legacy_parse() {
    let defaults = WebSearchSettings::default();
    assert_eq!(defaults.base_url, "");
    assert_eq!(defaults.timeout_ms, 15_000);
    assert_eq!(defaults.effective_timeout_ms(), 15_000);
    defaults.validate().unwrap();

    let configured = WebSearchSettings {
        base_url: "https://search.example.invalid/api".to_owned(),
        timeout_ms: 30_000,
    };
    configured.validate().unwrap();
    assert_eq!(
        configured,
        serde_json::from_str::<WebSearchSettings>(&serde_json::to_string(&configured).unwrap())
            .unwrap()
    );

    // The tool only speaks https; plain http/non-URL/base-less values are rejected.
    for base_url in [
        "http://search.example.invalid",
        "ftp://search.example.invalid",
        "search.example.invalid",
    ] {
        let invalid = WebSearchSettings {
            base_url: base_url.to_owned(),
            ..WebSearchSettings::default()
        };
        assert!(
            invalid.validate().is_err(),
            "scheme {base_url} must be rejected"
        );
    }

    // Timeout is bounded (UI + Ajv schema + Rust validation agree).
    for timeout_ms in [0, 999, 120_001] {
        let invalid = WebSearchSettings {
            timeout_ms,
            ..WebSearchSettings::default()
        };
        assert!(
            invalid.validate().is_err(),
            "timeout {timeout_ms} must be rejected"
        );
    }
    assert_eq!(
        WebSearchSettings {
            timeout_ms: 0,
            ..WebSearchSettings::default()
        }
        .effective_timeout_ms(),
        WebSearchSettings::MIN_TIMEOUT_MS
    );
    assert_eq!(
        WebSearchSettings {
            timeout_ms: u64::MAX,
            ..WebSearchSettings::default()
        }
        .effective_timeout_ms(),
        WebSearchSettings::MAX_TIMEOUT_MS
    );

    // AppSettings::validate delegates to the web-search section.
    let invalid_app = AppSettings {
        web_search: WebSearchSettings {
            base_url: "http://search.example.invalid".to_owned(),
            ..WebSearchSettings::default()
        },
        ..AppSettings::default()
    };
    assert!(invalid_app.validate().is_err());

    // Legacy settings JSON without `web_search` keeps backwards compatibility.
    let legacy: AppSettings = serde_json::from_str(
        r#"{"schema":1,"provider":{"kind":"mock"},"theme":"auto","locale":"zh-CN","auto_approve_read_only":true}"#,
    )
    .unwrap();
    assert_eq!(legacy.web_search, WebSearchSettings::default());
}

/// Third-party stores that only implement `load`/`save` keep working: the
/// revision methods are defaulted (`revision()=0`, fence ignored, returns `0`).
#[test]
fn settings_store_revision_defaults_keep_third_party_stores_compatible() {
    use std::sync::Mutex;
    use zk_contract_config::{ConfigError, SettingsStore};

    #[derive(Default)]
    struct MemoryStore {
        settings: Mutex<AppSettings>,
    }

    impl SettingsStore for MemoryStore {
        fn load(&self) -> Result<AppSettings, ConfigError> {
            Ok(self.settings.lock().unwrap().clone())
        }
        fn save(&self, settings: &AppSettings) -> Result<(), ConfigError> {
            *self.settings.lock().unwrap() = settings.clone();
            Ok(())
        }
    }

    let store = MemoryStore::default();
    assert_eq!(store.revision().unwrap(), 0);
    let mut next = AppSettings {
        theme: "dark".to_owned(),
        ..AppSettings::default()
    };
    next.locale = "en-US".to_owned();
    // A stale expected revision is ignored by the default implementation (legacy behaviour).
    assert_eq!(store.save_with_revision(&next, Some(41)).unwrap(), 0);
    assert_eq!(store.load().unwrap().theme, "dark");
    // The fence-mismatch error is a distinct, readable variant (host maps it to
    // `settings/revision-conflict`).
    assert_eq!(
        ConfigError::Conflict("stale".to_owned()).to_string(),
        "config revision conflict: stale"
    );
}

#[test]
fn approval_policy_truth_table_and_weak_invariant() {
    use ApprovalPolicy::{Ask, AutoReadOnly, Strict};

    // (requires_approval, mutating) -> expected ask per policy.
    let cases = [
        (true, true, [true, true, true]),
        (true, false, [true, true, true]),
        (false, true, [false, true, true]),
        (false, false, [false, false, true]),
    ];
    for (requires_approval, mutating, expected) in cases {
        assert_eq!(Ask.should_ask(requires_approval, mutating), expected[0]);
        assert_eq!(
            AutoReadOnly.should_ask(requires_approval, mutating),
            expected[1]
        );
        assert_eq!(Strict.should_ask(requires_approval, mutating), expected[2]);
    }
    // T1 weak invariant: a tool that declares approval is never auto-approved.
    for policy in [Ask, AutoReadOnly, Strict] {
        for mutating in [true, false] {
            assert!(
                policy.should_ask(true, mutating),
                "policy {policy:?} must ask for requires_approval tools"
            );
        }
    }
    // The four mutating tools that declare no approval are honestly not asked
    // under the default policy (schedule/job_kill/goal/mcp_call).
    assert!(!Ask.should_ask(false, true));
}

#[test]
fn approval_policy_wire_format_and_lenient_unknown_values() {
    use ApprovalPolicy::{Ask, AutoReadOnly, Strict};

    assert_eq!(Ask.as_str(), "ask");
    assert_eq!(AutoReadOnly.as_str(), "auto_read_only");
    assert_eq!(Strict.as_str(), "strict");
    assert!(serde_json::to_string(&ToolSettings {
        approval_policy: Some(Strict),
        ..ToolSettings::default()
    })
    .unwrap()
    .contains("\"approval_policy\":\"strict\""));

    // Unknown strings and invalid types degrade to `ask` instead of failing the
    // whole document (R5).
    let unknown: ToolSettings = serde_json::from_str(r#"{"approval_policy":"bogus"}"#).unwrap();
    assert_eq!(unknown.approval_policy, Some(Ask));
    let number: ToolSettings = serde_json::from_str(r#"{"approval_policy":42}"#).unwrap();
    assert_eq!(number.approval_policy, Some(Ask));
    let null: ToolSettings = serde_json::from_str(r#"{"approval_policy":null}"#).unwrap();
    assert_eq!(null.approval_policy, None);

    let mut settings = AppSettings::default();
    settings.tools.approval_policy = Some(Ask);
    assert_eq!(settings.effective_approval_policy(), Ask);
    settings.tools.approval_policy = Some(AutoReadOnly);
    assert_eq!(settings.effective_approval_policy(), AutoReadOnly);
    settings.tools.approval_policy = Some(Strict);
    assert_eq!(settings.effective_approval_policy(), Strict);
}

#[test]
fn approval_migration_and_normalization_are_idempotent() {
    use ApprovalPolicy::{Ask, AutoReadOnly, Strict};

    // No explicit policy + legacy default (true) -> ask; canonical form keeps the
    // policy absent so default-only documents stay unpublished.
    let mut settings = AppSettings::default();
    settings.normalize_approval_policy();
    assert_eq!(settings.effective_approval_policy(), Ask);
    assert_eq!(settings.tools.approval_policy, None);
    assert!(settings.auto_approve_read_only);

    // Legacy `false` (auto-approve off) -> strict.
    let mut settings = AppSettings {
        auto_approve_read_only: false,
        ..AppSettings::default()
    };
    settings.normalize_approval_policy();
    assert_eq!(settings.tools.approval_policy, Some(Strict));
    assert!(!settings.auto_approve_read_only);

    // Explicit policy wins over the legacy field; canonical mirror is written.
    let mut settings = AppSettings {
        auto_approve_read_only: false,
        ..AppSettings::default()
    };
    settings.tools.approval_policy = Some(Ask);
    settings.normalize_approval_policy();
    assert_eq!(settings.tools.approval_policy, None);
    assert!(settings.auto_approve_read_only);

    let mut settings = AppSettings::default();
    settings.tools.approval_policy = Some(AutoReadOnly);
    settings.normalize_approval_policy();
    assert_eq!(settings.tools.approval_policy, Some(AutoReadOnly));
    assert!(settings.auto_approve_read_only);

    // Idempotent: a second pass changes nothing.
    let once = settings.clone();
    settings.normalize_approval_policy();
    assert_eq!(settings, once);
}

#[test]
fn approval_migration_reads_legacy_json_shapes() {
    // 4 document shapes: absent tools / no policy field / explicit value / null.
    let absent = r#"{"schema":1,"provider":{"kind":"mock"},"theme":"auto","locale":"zh-CN","auto_approve_read_only":true}"#;
    let parsed: AppSettings = serde_json::from_str(absent).unwrap();
    assert_eq!(parsed.effective_approval_policy(), ApprovalPolicy::Ask);

    let no_policy = r#"{"schema":1,"provider":{"kind":"mock"},"theme":"auto","locale":"zh-CN","auto_approve_read_only":false,"tools":{"tool_timeout_ms":1000,"approval_timeout_ms":1000}}"#;
    let parsed: AppSettings = serde_json::from_str(no_policy).unwrap();
    assert_eq!(parsed.effective_approval_policy(), ApprovalPolicy::Strict);

    let explicit = r#"{"schema":1,"provider":{"kind":"mock"},"theme":"auto","locale":"zh-CN","auto_approve_read_only":false,"tools":{"tool_timeout_ms":1000,"approval_timeout_ms":1000,"approval_policy":"auto_read_only"}}"#;
    let parsed: AppSettings = serde_json::from_str(explicit).unwrap();
    assert_eq!(
        parsed.effective_approval_policy(),
        ApprovalPolicy::AutoReadOnly
    );

    let null_policy = r#"{"schema":1,"provider":{"kind":"mock"},"theme":"auto","locale":"zh-CN","auto_approve_read_only":true,"tools":{"tool_timeout_ms":1000,"approval_timeout_ms":1000,"approval_policy":null}}"#;
    let parsed: AppSettings = serde_json::from_str(null_policy).unwrap();
    assert_eq!(parsed.effective_approval_policy(), ApprovalPolicy::Ask);
}

fn local_server(id: &str) -> zk_contract_connector::McpServerConfig {
    zk_contract_connector::McpServerConfig {
        id: id.to_owned(),
        name: id.to_owned(),
        transport: zk_contract_connector::ConnectorTransport::Stdio,
        command: "mcp-demo".to_owned(),
        args: Vec::new(),
        env: std::collections::BTreeMap::new(),
        url: String::new(),
        timeout_ms: 0,
        oauth_required: false,
    }
}

#[test]
fn mcp_settings_defaults_roundtrip_and_validation() {
    use zk_contract_config::McpSettings;

    // Backward-compatible: an old document without `mcp` parses to the default.
    let legacy: AppSettings = serde_json::from_str(r#"{"schema":1,"provider":{"kind":"mock"},"theme":"auto","locale":"zh-CN","auto_approve_read_only":true}"#).unwrap();
    assert_eq!(legacy.mcp, McpSettings::default());
    assert_eq!(
        serde_json::to_value(AppSettings::default()).unwrap()["mcp"]["servers"],
        serde_json::json!([])
    );

    let mut settings = AppSettings::default();
    settings.mcp.servers = vec![local_server("mcp-demo")];
    settings.validate().unwrap();
    let json = serde_json::to_string(&settings).unwrap();
    let parsed: AppSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.mcp.servers[0].id, "mcp-demo");

    // Duplicate ids are rejected.
    settings.mcp.servers.push(local_server("mcp-demo"));
    assert!(settings.validate().is_err());

    // Id charset is local-only.
    settings.mcp.servers = vec![local_server("bad id/space")];
    assert!(settings.validate().is_err());

    // Caps: count / args / env / url / timeout.
    settings.mcp.servers = (0..=McpSettings::MAX_SERVERS)
        .map(|index| local_server(&format!("s{index}")))
        .collect();
    assert!(settings.validate().is_err());
    settings.mcp.servers = vec![local_server("ok")];
    settings.mcp.servers[0].args = vec!["x".to_owned(); McpSettings::MAX_ARGS + 1];
    assert!(settings.validate().is_err());
    settings.mcp.servers[0].args.clear();
    settings.mcp.servers[0].env.insert(
        "K".to_owned(),
        "v".repeat(McpSettings::MAX_ENV_VALUE_CHARS + 1),
    );
    assert!(settings.validate().is_err());
    settings.mcp.servers[0].env.clear();
    settings.mcp.servers[0].timeout_ms = McpSettings::MAX_TIMEOUT_MS + 1;
    assert!(settings.validate().is_err());

    // Local OAuth is unsupported.
    settings.mcp.servers[0].timeout_ms = 0;
    settings.mcp.servers[0].oauth_required = true;
    let error = settings.validate().unwrap_err().to_string();
    assert!(error.contains("mcp/validation"), "{error}");
}

#[test]
fn mcp_two_level_validation_catalog_legal_but_local_capped() {
    // A catalog-legal server can exceed local caps: `McpServerConfig::validate`
    // (catalog path) accepts it, while `McpSettings::validate` (local path) rejects it.
    let mut server = local_server("catalog-server-with-a-long-name");
    server.name = "n".repeat(200); // > local 128 cap, still catalog-legal
    server.validate().unwrap();
    let settings = zk_contract_config::McpSettings {
        servers: vec![server],
    };
    assert!(settings.validate().is_err());
}
