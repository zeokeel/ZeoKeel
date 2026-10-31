# stage2-media-config · 提案（配置完形第二步）

- 状态：`approved`
- Owner：lead（auditor-rig / auditor-web）
- 日期：2026-10-31
- 关联：docs/38 §5.2；objective 4；stage2-embedding-config（已 APPROVED）
- 评审：docs/reviews/2026-10-31-stage2-media-config.md
- 变更预算：image/audio/transcription/rerank 配置入 AppSettings + UI + glue + C7

## 1. 背景

媒体能力（image/audio/transcription/rerank）目前只能经 `ZEOKEEL_{IMAGE,AUDIO,TRANSCRIPTION,RERANK}_*` env 配置，设置页不可见；这是目标 4 的剩余隐藏配置。

## 2. 目标

- `AppSettings.media { base_url, api_key, image_model, audio_model, transcription_model, rerank_model }`（serde default；空 api_key = env fallback/no-op；base_url 若填必须 http(s)）。
- 生成 glue `configure_media_from_settings(&MediaSettings) -> Result<(),String>`：逐能力用 `zk_kit_media::MediaConfig::new(kind.default_provider(), base,key,model-or-default)` + `configure_from_config`；错误聚合返回；企业/no-media 生成 no-op。
- desktop boot/settings.set 应用/失败不落盘/agent_config_error；settings.get mask api_key；enterprise 拒绝/隐藏/boot 清理（C7）。
- 设置页“媒体生成/转写/重排”区（个人版可见）：base_url/api_key + 4 个 model 字段，i18n/validation/persist；企业版隐藏。
- docs/38/schema/TS/测试。

## 3. 非目标
- 真实 provider 调用（UNVERIFIED）；改变 rig features。
