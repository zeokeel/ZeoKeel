# 2026-10-31-round5-upgrade-and-e2e · 提案

- 状态：`approved`
- Owner：lead（auditor-rust / auditor-web）
- 日期：2026-10-31
- 关联：docs/04 §2.7/§6；docs/09 §7；docs/19；round-4 review §3
- 评审：docs/reviews/2026-10-31-round5-verification.md
- 变更预算：业务面全局 426 + 客户端强制升级 UX + changes/usage e2e + OEM pin gitlink

## 1. 背景

- round-4 已实现 bootstrap/channel/updates 的 426，但**业务面全局 426 层**仍缺（docs/04 §2.7/§6：低于 min_supported 时除稳定面/登录/刷新外一律 426）。
- 客户端未处理 426 `CLIENT_TOO_OLD`，无强制升级页（docs/19/09 要求展示强制升级并给门户入口）。
- workspace.changes/usage 重启只有单测，无真实 e2e 证据（round-4 review P2）。
- OEM pin 文档写 submodule gitlink，实现/夹具用目录哈希；需要生产 gitlink 支持 + 文档真源。

## 2. 目标

- 服务端对 `/api/client/v1/*`（排除 auth/login、auth/refresh、稳定面）按 `X-ZeoKeel-Client` 版本与 `min_client_version` 返回 426 `CLIENT_TOO_OLD`；测试覆盖放行/拒绝边界。
- 客户端 transport 识别 426 → 强制升级门禁（显示当前/最低版本、服务端 `/portal` 链接、不重试、不崩溃）；单测 + e2e。
- m1-serve e2e：真实 SQLite 下 usage 重启可见；真实 git 临时仓库 inspector changes tab 有数据。
- OEM pin：索引目录是 git 仓库时用 submodule gitlink hash 校验；fixture/非 git 目录回退目录哈希并显式记录；docs/09 更新。
- 全量门禁 + C19 APPROVED。

## 3. 非目标

- 真实签名密钥/证书、Docker/多平台、`zk api` 路由真源（继续 backlog/UNVERIFIED）。
- 子代理异步 send/interrupt（下一轮）。

## 4. 成功指标

- 426 gate 单测/集成绿；客户端 426 单测 + e2e；usage/changes e2e 绿；OEM 校验生产/夹具两条路径测试；门禁全绿。
