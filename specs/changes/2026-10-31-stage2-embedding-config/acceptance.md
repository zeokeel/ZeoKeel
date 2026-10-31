# stage2-embedding-config · 验收

- 状态：`done`（C19 终评 APPROVED at HEAD b927efe；首评 P1=2 已修复）
- 验收人：lead；C19 独立评审：auditor-build

## 1. 结果

| 命令 | 通过条件 |
| --- | --- |
| cargo fmt/clippy/test --workspace | 0 |
| zk/schema/docs gates | 0 |
| pnpm gates | 0 |
| embedding 契约/glue/desktop/UI 测试 | 通过 |
| C19 评审 | APPROVED（auditor-build 独立；P0/P1=0；P2 见 review §10） | docs/reviews/2026-10-31-stage2-embedding-config.md |

## 1.1 实现与复评

- EmbeddingSettings（base_url/api_key/model/dims/dir）+ AppSettings.embedding；schema/TS、密钥 masking/保留、Debug 脱敏、dims 1..65536。
- 生成 glue `configure_embedding_from_settings -> Result`：settings > env > 未配置；空 key no-op；dir 持久化；失败传播。
- desktop settings.set 应用失败即返回错误且不落盘；boot 失败写 agent_config_error；enterprise 拒绝/隐藏/启动清理 stale embedding（C7）。
- 设置页“嵌入/向量索引”个人版可见、企业版隐藏；测试 150（desktop 109）。
- 非阻塞 P2：embedding 在 agent/provider 保存前生效的部分状态、企业 clear 保存失败仅日志、部分组合显式测试缺失。

## 2. UNVERIFIED
- 真实 embedding provider；媒体 image/audio/transcription/rerank 配置（下一阶段）；Tauri/Windows；Docker。
