# stage2-embedding-config · 提案（下一阶段：配置完形第一步）

- 状态：`approved`
- Owner：lead（auditor-rig / auditor-web）
- 日期：2026-10-31
- 关联：docs/38 §5.2/§5.4；objective 4（所有项目可配置）
- 评审：docs/reviews/2026-10-31-stage2-embedding-config.md
- 变更预算：embedding/vector 配置入 AppSettings + 设置 UI + 运行时应用

## 1. 背景

embedding/vector 当前只能通过 `ZEOKEEL_EMBEDDING_*` / `ZEOKEEL_VECTOR_DIR` env 配置（个人版 boot），设置页不可见、修改需重启环境变量；这是目标 4 的隐藏配置。媒体 image/audio/transcription/rerank 仍 env-only，留到下一步。

## 2. 目标

- `AppSettings.embedding { base_url, api_key, model, dims: Option<u64>, dir }`（serde default；validate：dims>0、base_url 若填必须 http(s)、dir 无约束）。
- 生成 glue 新增 `zk_composed::configure_embedding_from_settings(&EmbeddingSettings)`：api_key 非空时构造 `zk_kit_media::EmbeddingConfig` + `embedding_model_from_config`，调用 `zk_cap_vector::configure_with_dir(model, dir)`；空配置 no-op（保留 env fallback）。
- desktop boot（settings 文件存在）与 settings.set 应用；错误回传 UI；不发布到 RuntimeSettings（密钥不进入 agent 侧）。
- 设置页“嵌入/向量索引”区：base_url/api_key/model/dims/dir，范围/URL 校验、i18n、持久化；旧设置兼容。
- docs/38 更新配置优先级（settings > env > unavailable）；schema/TS 重生成；测试（contract/glue/desktop/web）。

## 3. 非目标
- media image/audio/transcription/rerank 配置（下一阶段）；真实 provider 调用 UNVERIFIED。
