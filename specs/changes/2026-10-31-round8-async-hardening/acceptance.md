# 2026-10-31-round8-async-hardening · 验收

- 状态：`in-progress`
- 验收人：lead；C19 独立评审：auditor-build

## 1. 验收结果

| 命令 | 通过条件 |
| --- | --- |
| cargo fmt/clippy/test --workspace | 0 |
| zk/ compose/schema/docs/db lock/lint/matrix | 0 |
| pnpm gates | 0 |
| e2e agent/m1/login/admin/client-upgrade | 0 |
| async 压力/panic/竞态、request-id 测试 | 通过 |
| C19 评审 | APPROVED |

## 1.1 实现记录

- async：存活线程 RAII 计数、catch_unwind、CAS 状态机（interrupt 优先）、children/kit registry 64 有界；压力/panic/竞态测试。
- request-id：最外层 middleware 一次生成/回显，gate/bootstrap/updates/gateway settlement/audit 复用；真实 PG 验证。
- gateway：GatewayLlm 带 client header；但 `/v1/*` 的 426 门禁因 rig-core 原生 Responses 客户端不支持自定义头而**不启用**（docs/04 例外；JWT+发布锁定），列为后续上游依赖。
- matrix writer：docs/24 归属 scripts，CI/check.sh 增加 `--check`。

## 2. UNVERIFIED

- Tauri 真机/Windows、真实模型长时取消、Docker/多平台、cargo-deny 本地、真实私有 OEM。
