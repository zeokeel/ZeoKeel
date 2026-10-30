# cap-rerank

**功能说明**：rerank（media.rerank） 能力 —— 本轮**诚实不可用**：rig rerank 已在 core 暴露，但未配置 rerank provider。

- `available() == false`；`unavailable_reason()` 返回具体原因；调用方统一映射
  `CAPABILITY_UNAVAILABLE`，不得静默丢失能力。
- 未来接入 provider 后由 host 调用 `set_available(true)`；rig 的 image/audio 还需在
  `Cargo.toml` 启用对应 feature，并新增 provider 适配包。
