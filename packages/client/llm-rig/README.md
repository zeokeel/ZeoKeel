# zk-llm-rig (`llm.provider.default`)

**功能说明**：把 rig 的模型 provider 暴露给 agent，而不是自研 LLM 客户端。当前支持 OpenAI-compatible/DeepSeek 兼容端点（base_url + api_key + model）；企业网关仍走 Responses 兼容包。模型类型为 `rig_core::DynModel<Completion>`，agent 用 rig 原生 tool calling。
