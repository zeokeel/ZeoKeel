# 2026-10-31-round10-truth-and-p2 · 提案

- 状态：`approved`
- Owner：lead（auditor-rig / auditor-web / auditor-rust）
- 日期：2026-10-31
- 关联：round-9 review §8.3；docs/03 §6.1；docs/24 §3/§4；AGENTS §6
- 评审：docs/reviews/2026-10-31-round10-verification.md
- 变更预算：文档真源（zk api/lock/metadata）+ P2 收尾 + 调度/异步 e2e

## 1. 背景

- docs/03 §6.1、AGENTS §6、docs/24 §3 列出的 `zk api`、`zk verify-lock`、`zk lock`、`zk metadata snapshot` **命令不存在**，CI 也未执行；属文档/门禁 false claim，必须要么实现、要么明确标为未实现并从 gate 清单移除。
- round-9 P2：docs/19 无 schedule 说明；热更新无真实 e2e；async 缺真实并发/spawn-failure、retention 文档；request-id 客户端值字符集过宽；docs/04 末句“全链路透传”与 gateway settlement id 区分不清。
- 本轮选择：**不假装实现 `zk api` 全家桶**（需 manifest 路由声明 + runtime Register 对账，工作量大且易做成半成品），而是把文档/CI/gate 清单改为真实状态（显式 backlog），同时补齐可执行的 P2 加固与 e2e。

## 2. 目标

- docs/03/AGENTS/docs/24/generated README：`zk api`/`zk lock`/`zk verify-lock`/`zk metadata snapshot` 标记 `未实现（backlog）`，从“当前 gates/CI 已执行”清单移除；CI/AGENTS 只列可执行命令。
- docs/19 补 schedule 设置说明；docs/38 补 async retention 淘汰语义（completed 被淘汰后 status NotFound）。
- request-id：`X-Request-Id` 仅接受安全字符集（如 `[A-Za-z0-9._-]{1,128}`），非法则生成新 id；gate/bootstrap/updates/audit 一致；docs/04 明确 HTTP request_id 全链路关联、gateway settlement id 独立。
- async：补 spawn-failure/真实线程并发压力测试（非 TestHooks）；可选 async e2e。
- schedule：m1-serve e2e 覆盖设置 UI 保存/重载后 schedule 值持久化（热更新执行器行为继续单测；真实自动 turn e2e 若可行）。
- 全量门禁 + C19 APPROVED。

## 3. 非目标
- 实现 `zk api` 路由声明/运行时对账（显式 backlog，见 docs/03/24）。
- M6 Docker/多平台、Tauri 真机。
