# cap-rerank

**功能说明**：文档重排（`media.rerank`）能力 —— 通过 rig-core rerank 模型实现。rig-core 0.45 的
OpenAI wire **没有 rerank 端点**（`RerankQuirks::unsupported`），唯一自带 rerank wire 的 provider 是
Voyage AI，因此本包默认/仅支持 `provider = voyageai`；`openai` 会返回明确的配置错误而不是假装可用。
**动态可用**：没有配置 rerank 模型时 `available() == false`，`unavailable_reason()` 给出具体原因；
调用方统一映射 `CAPABILITY_UNAVAILABLE`，不得静默丢失能力。

实现位于共享 rig 适配层 `crates/zk-kit-media`（`kit.media`）；本包只依赖该 kit + 自己的结果 DTO，
不直接依赖 `rig-core` / `zk-llm-rig`（C4/C17）。测试使用 kit 的 `test-support` 假模型（无网络）。

- API：`configure(model)` / `configure_from_config(&MediaConfig)` / `configure_from_env()` /
  `unconfigure()`；`rerank(query, documents)` 返回按相关性排序的
  `RerankHit { index, score, document }`。
- 配置（env）：`ZEOKEEL_RERANK_PROVIDER`（默认 `voyageai`）、`ZEOKEEL_RERANK_BASE_URL`、
  `ZEOKEEL_RERANK_API_KEY`（必填）、`ZEOKEEL_RERANK_MODEL`（默认 `rerank-2.5`）。
- 企业版：personal boot 才调用 `zk_composed::configure_capabilities_from_env()`（进而调用本包
  `configure_from_env()`）；enterprise boot 不配置媒体（网关只支持 Responses，C7/ADR-031），
  能力保持不可用并给出原因。
