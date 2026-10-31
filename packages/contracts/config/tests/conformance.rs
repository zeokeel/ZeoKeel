//! Contract conformance: settings round-trip and defaults.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use zk_contract_config::{
    AgentSettings, AppSettings, EmbeddingSettings, MediaSettings, ProviderKind, RuntimeSettings,
    ScheduleSettings, ToolChoiceSetting, ToolSettings,
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

    let snapshot = RuntimeSettings {
        agent: Some(settings.agent.clone()),
        tools: Some(settings.tools),
    };
    snapshot.validate().unwrap();
    zk_contract_config::set_runtime_settings(&snapshot).unwrap();
    let published = zk_contract_config::runtime_settings().unwrap().unwrap();
    assert_eq!(
        published.agent.as_ref().unwrap().completion.temperature,
        Some(0.3)
    );
    assert_eq!(published.tools.unwrap().tool_timeout_ms, 2_000);

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
