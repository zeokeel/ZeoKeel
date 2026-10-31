# stage2-media-config · 规格

- 状态：`approved`

## 1. contract
`MediaSettings { base_url="", api_key="", image_model="", audio_model="", transcription_model="", rerank_model="" }` Default; `AppSettings.media`; validate base_url http(s) when non-empty; `is_configured` requires api_key non-empty; Debug redacts key.
## 2. glue/runtime
`configure_media_from_settings(&MediaSettings) -> Result<(),String>`:
- if api_key empty → Ok(()) (env fallback unchanged)
- for each kind: model = settings.<kind>_model or kind.default_model(); config = MediaConfig::new(kind.default_provider(), base, key, model); `zk_cap_<kind>::configure_from_config(&config)`; collect first error (or aggregate) and return Err; partial successes remain configured (document).
- enterprise/no-media build → `Ok(())` no-op.
Desktop apply before save; error blocks save; boot failure agent_config_error; enterprise reject configured + boot clear stale (save sanitized).
## 3. UI
Settings `showMedia` prop personal-only; fields base url, api key, image/audio/transcription/rerank model; validation/i18n; old settings defaults.
## 4. tests/docs
contract/glue/desktop/UI tests; docs/38 priority/masking/personal-only; schema/TS.
