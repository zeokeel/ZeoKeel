# cap-audio

**功能说明**：语音合成（`media.audio`）能力 —— 通过 rig-core OpenAI 兼容 `audio_generation`（TTS）模型实现
（workspace 已启用 rig `audio` feature）。**动态可用**：没有配置音频模型时 `available() == false`，
`unavailable_reason()` 给出具体原因；调用方统一映射 `CAPABILITY_UNAVAILABLE`，不得静默丢失能力。

- API：`configure(model)` / `configure_from_config(&MediaConfig)` / `configure_from_env()` /
  `unconfigure()`；`synthesize(text, voice)` 返回 `GeneratedAudio { bytes, provider, model }`。
- 配置（env）：`ZEOKEEL_AUDIO_PROVIDER`（默认 `openai`）、`ZEOKEEL_AUDIO_BASE_URL`、
  `ZEOKEEL_AUDIO_API_KEY`（必填）、`ZEOKEEL_AUDIO_MODEL`（默认 `tts-1`）。
- 不保存资产：音频字节由调用方接管（落盘/播放自定）。
- 企业版：网关只支持 Responses（C7），不提供语音端点，能力保持不可用并给出原因。
