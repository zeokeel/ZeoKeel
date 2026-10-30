# 2026-10-31-config-and-media · 验收

- 状态：`in-progress`
- 验收人：lead；修复：auditor-rig / auditor-web / fixer-media
- 证据目录：`target/evidence/config-media/`、各 crate 测试日志

## 1. 待填验证

| 命令 | 通过条件 | 证据 |
| --- | --- | --- |
| `cargo fmt/clippy/test --workspace` | 全绿 | 日志 |
| `cargo test --workspace export_schema && git diff --exit-code` | schema 稳定 | diff |
| `bash scripts/schema-check.sh` | 绿 | 日志 |
| `pnpm -r typecheck/lint/test:unit/build` | 全绿 | 日志 |
| `zk compose personal --check` / capability / docs / db lock | 全绿 | 日志 |
| `scripts/{login,admin,agent,m1}-e2e` | 全绿 | 证据 |
| 设置持久化/生效、usage、媒体能力、Ajv 负向 | 单测通过 | 日志 |

## 2. UNVERIFIED

- 真实媒体 provider（仅 fake/local model 验证编解码）；企业媒体端点（不在本期）。
- Docker/网络/多平台沿用上一轮 UNVERIFIED。

## 3. 剩余

完成后回填证据与状态。
