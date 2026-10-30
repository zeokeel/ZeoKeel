# 2026-10-31-config-and-media · 验收

- 状态：`done`（C19 复评 APPROVED，HEAD 9ed7d40；e2e 全绿）
- 验收人：lead；修复：auditor-rig / auditor-web / fixer-media
- 证据目录：`target/evidence/config-media/`、各 crate 测试日志

## 1. 待填验证

| 命令 | 通过条件 | 证据 |
| --- | --- | --- |
| `cargo fmt/clippy/test --workspace` | 全绿（本机已跑：0/0/0） | 见提交与门禁日志 |
| `cargo test --workspace export_schema` + `schema-check.sh` | 0（已跑） | shared/contracts/schema,ts |
| `bash scripts/schema-check.sh` | 0（已跑） | 日志 |
| `pnpm -r typecheck/lint/test:unit/build` | 0/0/0/0（92 tests，已跑） | 日志 |
| `zk compose personal --check` / capability / docs / db lock | 0/0/0/0（capability 45） | 日志 |
| `scripts/{login,admin,agent,m1}-e2e` | login 4 passed（1 first-run + 3 场景）· admin 11 passed · agent 9 passed ×2 · M1 14 passed/15 skipped | target/evidence/ |
| 设置持久化/生效、usage、媒体能力、Ajv 负向 | 单测通过（contract/agent-rig/desktop/web/kit-media 共 150+ tests；Ajv 13 条含 unknown keyword/format） | 各 crate tests；target/evidence/config-media/ |

## 2. UNVERIFIED

- 真实媒体 provider（仅 fake/local model 验证编解码）；企业媒体端点（不在本期）。
- Docker/网络/多平台沿用上一轮 UNVERIFIED。

## 3. 剩余

完成后回填证据与状态。
