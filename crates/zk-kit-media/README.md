# zk-kit-media

**功能说明**：rig-core 0.45 媒体模型的唯一共享适配层（`kit.media`），供
`cap-{image,audio,transcription,rerank}` 复用，避免能力包直接依赖 `rig-core` / `llm-rig`（C4/C17）。

- 模型：image `image_generation`、audio `audio_generation`（TTS）、transcription、rerank
  （rig-core 0.45 OpenAI wire 无 rerank 端点，rerank 走 Voyage AI），以及 OpenAI-compatible embeddings
  （`EmbeddingConfig`/`EmbeddingModel`/`embedding_model[_from_config]`/`embed_text`，供 `cap-vector`）。
- 配置：`MediaConfig` + `MediaKind`，env 前缀 `ZEOKEEL_{IMAGE,AUDIO,TRANSCRIPTION,RERANK}_*`
  （`PROVIDER` / `BASE_URL` / `API_KEY` / `MODEL`）。`MediaConfig` 的 `Debug` 会**脱敏** `api_key`。
- builder：`image_model[_from_config]()` / `audio_model[_from_config]()` /
  `transcription_model[_from_config]()` / `rerank_model[_from_config]()`，返回不透明句柄
  （`ImageModel` 等），cap-* 不再直接看到 rig 类型。
- 调用：`generate_image` / `synthesize_audio` / `transcribe_audio` / `rerank_documents`；
  `block_on` 是同步能力 API 与 rig async 模型之间唯一的 runtime 桥。
- 槽位：`ModelSlot<M>` 统一 cap-* 的 configure/unconfigure/available 结构，避免四份复制。
- 测试：`test_support` 模块（feature `test-support`，crate 自身测试自动启用）提供确定性
  进程内假模型，无网络、无 provider。
- feature 前提：workspace 只启用 rig `audio`/`image`；`pdf`（依赖未缓存）与 `epub`
  （GPL-3.0 与 MIT 冲突）不启用。
