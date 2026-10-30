# 2026-10-31-round6-hardening · 验收

- 状态：`in-progress`（实现完成；C19 复评待）
- 验收人：lead；C19 独立评审：auditor-build（非实现者）

## 1. 验收结果

| 命令 | 通过条件 |
| --- | --- |
| cargo fmt/clippy/test --workspace | 0 |
| zk validate/compose personal --check/capability/docs/db lock/lint duplicates | 0 |
| pnpm -r typecheck/lint/test:unit/build | 0 |
| m1/agent/login/admin e2e + client-upgrade e2e | 全绿 |
| docs/spec/review gates | 0 |
| 426 真实 e2e | `scripts/client-upgrade-e2e.sh` 0（gate 显示 0.1.0/9.9.9、portal、无重试） |
| 426 envelope/渠道/request-id、OEM pin、Tauri CSP/open_url | 单测+真实 PG smoke/oem e2e 0 |
| C19 评审 | APPROVED |

## 1.1 实现记录

- web：`serverFetch` 统一所有 server 直连（client header + X-Request-Id + 426→UpgradeRequiredError），相对 portal 解析、版本说明、platform 归一、changes 文件级文案、Tauri 外部打开分支；真实 426 升级 e2e。
- server：426 统一信封（含 request_id/retryable/x-request-id 回显）、渠道级 min_client_version（`<CHANNEL>` override → brand → global）、CORS request-id。
- OEM：git 索引强制 gitlink；仅非 git fixture 回退目录哈希；4 个 channel_validate 测试 + oem-e2e。
- Tauri：CSP connect-src 允许企业 https + 本地回环；`system.open_url` 仅 http/https、固定 argv、无等待；能力不新增 ACL（自定义命令）。
- changes：UI 标注文件级 +add/−del（无行级 diff），docs/27/21/19/09/04 同步。

## 2. UNVERIFIED

- 真实 OEM 私有索引/submodule（fixture fallback 路径覆盖）；Tauri 真机（CSP/外链代码测试，不含显示环境）；cargo-deny 本地；Docker/多平台。
