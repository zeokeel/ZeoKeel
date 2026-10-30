# 2026-10-31-round7-async-and-determinism · 提案

- 状态：`approved`
- Owner：lead（auditor-rig / auditor-rust / auditor-web）
- 日期：2026-10-31
- 关联：round-6 review §8.4；docs/25 T20；docs/38；docs/04 §7
- 评审：docs/reviews/2026-10-31-round7-verification.md
- 变更预算：异步子代理 + 生成物确定性 + 426 envelope 残留 + open_url 收尾

## 1. 背景

- 目标 2 明确"子代理"功能；当前矩阵 subagent/subagent-control 为 partial：只有同步 run，无异步 start/status/interrupt（round-5/6 反复 deferred）。
- P2-N1：`zk` 生成 JSON 的 key 顺序随构建图变化，本地 `target/debug/zk` 可 false-red compose --check（round-6 review 记录）。
- 426 envelope 残留：bootstrap/updates 仍 flat，与 docs/04 §7 不一致；request_id 未落审计。
- `system.open_url` 子进程未 reap、失败无 UI 反馈（review §8.4）。

## 2. 目标

- 异步子代理：`subagent_start` 返回 child id；`subagent_status` 查状态/结果；`subagent_interrupt` 取消；agent-rig runner 支持后台运行与真正取消；工具矩阵/文档/测试更新；同步 `subagent`/`subagent_fork` 保持兼容。
- 生成物确定性：`zk` 所有 JSON 输出按 key 排序（BTreeMap/递归 stable_json），不同构建图下生成结果字节一致；加回归测试。
- 426：bootstrap/updates 的 426 使用统一嵌套 envelope（code/message/retryable/request_id/current/minimum/portal_url）；request_id 写入审计（若列已存在）；客户端兼容。
- open_url：后台线程 reap 子进程，失败通过 IPC 返回/UI toast；web 显示打开失败。
- 全量门禁 + C19 APPROVED。

## 3. 非目标
- gateway `/v1` 426 覆盖（记录为后续）；zk api；M6。
