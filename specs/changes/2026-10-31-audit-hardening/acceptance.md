# 2026-10-31-audit-hardening · 验收

- 状态：`done`（可验证项全绿；环境项 UNVERIFIED 见 §2）
- 验收人：lead（Agent Teams）；独立只读审计：auditor-rust/web/build/rig
- 证据目录：`target/evidence/audit/`、`target/evidence/fix/`、`target/evidence/{m1,m2,agent-rig}/`
- 基线：起点 257bae1；终点 9d3ed47（本 spec 随最后提交）

## 1. 自动验证（本机实跑，exit code）

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 |
| `cargo test --workspace` | 0 |
| `pnpm -r typecheck` / `lint` / `test:unit` / `build` | 0 / 0 / 0 / 0 |
| `zk validate` / `compose personal --check` / `capability --check` | 0 / 0 / 0 |
| `zk docs --check` / `db lock --check` | 0 / 0 |
| `python3 scripts/{docs-check,spec-check,review-check}.py` | 0 / 0 / 0 |
| `bash scripts/schema-check.sh` | 0 |
| enterprise `zk compose` + `cargo check -p zk-server -p zk-desktop -p zk-composed` | 0 |
| enterprise `pnpm build` desktop/admin | 0 / 0 |
| `bash scripts/login-e2e.sh` | 0（first-run 1 + 3 真实 server 场景） |
| `bash scripts/admin-e2e.sh` | 0（11 passed） |
| `bash scripts/agent-e2e.sh` | 0（9 passed，连续两次） |
| `bash scripts/m1-e2e.sh` | 0（14 passed / 15 skipped） |
| `bash scripts/check.sh` | 0 |

## 2. UNVERIFIED（环境阻塞，不得写已验证）

- Docker 镜像 build/save/load/compose、Caddy TLS 三模式、真实 upgrade/rollback/restore、PITR。
- 三平台安装包、Windows 签名（已决策不做）、macOS 公证；SBOM/可复现构建镜像级。
- 真实上游 OpenAI Responses（只用本地 fake SSE 验证编解码/工具/usage）；100 并发压测。
- 系统密钥库、上游 provider key master-key 加密（基线仍明文 `api_key_enc`，API 掩码）。

## 3. 剩余 backlog（下一轮）

1. T22 前端：桌面用生成 `SCHEMAS` + Ajv 替换 `protocol.ts` 手写 guard；删除 `ui-kit/types.ts` 镜像。
2. `zk api` 路由静态真源（`[[routes]]`→catalog/openapi/permissions/errors）、`zk lock/verify-lock`、`metadata snapshot`、`zk lint --duplicates` 实装。
3. rig 配置接线到 `AppSettings.agent` + 设置 UI；`AgentReply.usage` 展示/入库；logout reset rig model。
4. 企业 embeddings/vector 路由；媒体 provider（image/audio/transcribe/rerank）与 feature flag。
5. `docs/24` §1.5 角色表与实现对齐（含 `update.client`→`client.update`）；admin i18n、ui-sessions 重复命令面板（C17）、inspector changes 数据源、token 系统密钥库。
6. 内核收尾：desktop schedule std 线程可中止、server jobs cancellable token、Tauri setup 事件转发。
7. M6 证据链（Docker/perf/SBOM/axe 全矩阵）与 M0/M6 acceptance 归档；CI 增加 postgres 集成 job。

## 4. 说明

- 环境不可验证项已由脚本显式 UNVERIFIED/exit 2，不再 false-green。
- 本次为跨域加固，按域提交 12 个 Conventional Commits；审计报告与修复证据均在 `target/evidence/`。
