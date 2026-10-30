# cap-image

**功能说明**：图像生成（`media.image`）能力 —— 通过 rig-core OpenAI 兼容 `image_generation` 模型实现
（workspace 已启用 rig `image` feature）。**动态可用**：没有配置图像模型时 `available() == false`，
`unavailable_reason()` 给出具体原因；调用方统一映射 `CAPABILITY_UNAVAILABLE`，不得静默丢失能力。

实现位于共享 rig 适配层 `crates/zk-kit-media`（`kit.media`）；本包只依赖该 kit + 自己的结果 DTO，
不直接依赖 `rig-core` / `zk-llm-rig`（C4/C17）。测试使用 kit 的 `test-support` 假模型（无网络）。

- API：`configure(model)` / `configure_from_config(&MediaConfig)` / `configure_from_env()` /
  `unconfigure()`；`generate(prompt, width, height)` 返回 `GeneratedImage { bytes, encoding, provider, model }`。
- 配置（env）：`ZEOKEEL_IMAGE_PROVIDER`（默认 `openai`）、`ZEOKEEL_IMAGE_BASE_URL`、
  `ZEOKEEL_IMAGE_API_KEY`（必填）、`ZEOKEEL_IMAGE_MODEL`（默认 `gpt-image-1`）。
- 不保存资产：图片字节由调用方接管（落盘/展示自定），包内只做编码探测（PNG/JPEG/WebP/GIF）。
- 企业版：personal boot 才调用 `zk_composed::configure_capabilities_from_env()`（进而调用本包
  `configure_from_env()`）；enterprise boot 不配置媒体（网关只支持 Responses，C7/ADR-031），
  能力保持不可用并给出原因。
