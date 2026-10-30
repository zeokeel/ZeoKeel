# zk-llm-rig (`llm.provider.default`)

**功能说明**：把 rig 的模型 provider 暴露给 agent，而不是自研 LLM 客户端。当前支持
OpenAI-compatible/DeepSeek 兼容端点（base_url + api_key + model）；企业网关仍走 Responses 兼容包。
模型类型为 `rig_core::DynModel<Completion>`，agent 用 rig 原生 tool calling。

## 媒体模型（round 4，2026-10-31）

- 除 completion 外提供四个媒体 builder（返回 `rig_core::DynModel<Op>`）：
  - `image_model()` / `image_model_from_config(&MediaConfig)` → `ImageGeneration`
  - `audio_model()` / `audio_model_from_config(&MediaConfig)` → `AudioGeneration`（TTS）
  - `transcription_model()` / `transcription_model_from_config(&MediaConfig)` → `Transcription`
  - `rerank_model()` / `rerank_model_from_config(&MediaConfig)` → `Rerank`
- 配置：`MediaConfig` + `MediaKind`，env 前缀 `ZEOKEEL_{IMAGE,AUDIO,TRANSCRIPTION,RERANK}_*`
  （`PROVIDER` / `BASE_URL` / `API_KEY` / `MODEL`）。未配置（`*_API_KEY` 为空）返回
  `MediaError::Unconfigured`；配置非法返回 `MediaError::Config`。
- provider 支持：image/audio/transcription = `openai`（OpenAI 兼容端点）；rerank = `voyageai`
  （rig-core 0.45 无 OpenAI rerank 端点）。企业网关不提供这些端点（C7）。
- `block_on(future)`：cap-* 同步 API 与 rig async 模型的唯一共享桥；cap-* 不再各自复制 tokio runtime。
- rig features：workspace 只启用 `audio`/`image`；`pdf`（依赖未缓存、离线构建不可用）与 `epub`
  （GPL-3.0 与 MIT 冲突）均未启用，PDF/EPUB 解析由 `tool-read-document` 独立实现。
