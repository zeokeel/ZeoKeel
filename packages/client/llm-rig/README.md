# zk-llm-rig (`llm.provider.default`)

**功能说明**：把 rig 的模型 provider 暴露给 agent，而不是自研 LLM 客户端。当前支持
OpenAI-compatible/DeepSeek 兼容端点（base_url + api_key + model）；企业网关仍走 Responses 兼容包。
模型类型为 `rig_core::DynModel<Completion>`，agent 用 rig 原生 tool calling。

- 媒体模型（image/audio/transcription/rerank）与 OpenAI-compatible embedding 模型**不在本包**：
  它们是共享 rig 适配层 `crates/zk-kit-media`（`kit.media`），由 `cap-{image,audio,transcription,rerank}`
  与 `cap-vector` 消费；避免能力包直接依赖本实现包与 `rig-core`（C4/C17）。
- rig features：workspace 只启用 `audio`/`image`；`pdf`（需要未缓存的 `lopdf 0.44`，workspace 为
  `tool-read-document` 固定 `lopdf = "=0.42.0"`）与 `epub`（GPL-3.0 与 MIT 冲突）均不启用。
