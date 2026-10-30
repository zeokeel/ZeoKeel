# cap-transcription

**功能说明**：音频转写（`media.transcription`）能力 —— 通过 rig-core OpenAI 兼容 `transcription` 模型实现。
**动态可用**：没有配置转写模型时 `available() == false`，`unavailable_reason()` 给出具体原因；
调用方统一映射 `CAPABILITY_UNAVAILABLE`，不得静默丢失能力。

- API：`configure(model)` / `configure_from_config(&MediaConfig)` / `configure_from_env()` /
  `unconfigure()`；`transcribe(bytes, filename)` 与 `transcribe_file(path)` 返回
  `Transcript { text, language, duration, provider, model }`。
- 配置（env）：`ZEOKEEL_TRANSCRIPTION_PROVIDER`（默认 `openai`）、`ZEOKEEL_TRANSCRIPTION_BASE_URL`、
  `ZEOKEEL_TRANSCRIPTION_API_KEY`（必填）、`ZEOKEEL_TRANSCRIPTION_MODEL`（默认 `whisper-1`）。
- 不保存音频：调用方传字节或路径，包内读取后交给 provider。
- 企业版：网关只支持 Responses（C7），不提供转写端点，能力保持不可用并给出原因。
