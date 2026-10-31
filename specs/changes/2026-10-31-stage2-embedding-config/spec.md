# stage2-embedding-config · 规格

- 状态：`approved`

## 1. contract
`EmbeddingSettings { base_url: String="", api_key: String="", model: String="", dims: Option<u64>, dir: String="" }` with Default; `AppSettings.embedding` serde default; `AppSettings::validate` validates dims (1..=65536) and base_url scheme if non-empty.
## 2. glue/runtime
Generated composed crate exposes `configure_embedding_from_settings(settings: &zk_contract_config::EmbeddingSettings)`:
- if api_key trimmed empty → return (no-op, env fallback remains)
- build `EmbeddingConfig::new(base_url, api_key, model, dims usize)`; `embedding_model_from_config`; `zk_cap_vector::configure_with_dir(model, dir)`; errors eprintln with prefix (same style as configure_capabilities_from_env).
Desktop main boot (personal only, settings file exists) calls it after env configure; settings.set applies before save and surfaces errors; enterprise never (C7).
## 3. UI
settings section fields; api_key masked/returned? settings.get masks provider key already; mirror semantics (write-only? existing provider behavior). Use existing pattern: api_key input, masked on load via settings.get (empty), save sends value only if changed? Simpler same as provider: store raw; settings.get returns empty (masked) — need read existing UI pattern. Follow ui-settings-provider.
## 4. docs/tests
docs/38 update; contract conformance/schema; glue unit test (no-op empty); desktop apply test; web round-trip/validation.
