//! Contract conformance: settings round-trip and defaults.
#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only

use zk_contract_config::{AgentSettings, AppSettings, ProviderKind, ToolChoiceSetting};

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
