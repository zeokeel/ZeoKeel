# 2026-10-31-config-and-media · 任务

| # | 任务 | 验证 | 状态 |
| --- | --- | --- | --- |
| 1 | 契约 AppSettings.agent / AgentReply.usage / configure_agent | contract tests + export_schema 稳定 | done |
| 2 | 桌面应用 agent 配置（启动/settings.set/错误回传） | Rust 单测 + serve 冒烟 | done |
| 3 | agent-rig 填充 usage + gateway logout 重置占位 | cargo test | done |
| 4 | 生成 schema/TS 并更新 schema-check | `cargo test export_schema` / `schema-check.sh` | done |
| 5 | 设置 UI：agent completion/memory/hooks/max_steps + usage 展示 | pnpm 门禁 + vitest | done |
| 6 | 媒体能力 cap-image/audio/transcription/rerank + llm-rig builders | 各 crate tests + docs/38 | done |
| 7 | T22 桌面 Ajv 边界 + 负向测试 | vitest + build | done |
| 8 | 全量门禁 + e2e 复评 + 证据 | fmt/clippy/test/zk/docs/web/e2e | pending |
