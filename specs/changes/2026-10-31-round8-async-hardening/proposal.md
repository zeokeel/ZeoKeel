# 2026-10-31-round8-async-hardening · 提案

- 状态：`approved`
- Owner：lead（auditor-rig / auditor-rust）
- 日期：2026-10-31
- 关联：round-7 review §4 P2-1..4/7；docs/04 §7；docs/21；docs/24 §3
- 评审：docs/reviews/2026-10-31-round8-verification.md
- 变更预算：异步子代理加固 + request-id 一致性/gateway 426 + matrix/writer 真源

## 1. 背景

round-7 APPROVED，P2 中直接影响"完美/不限隐藏缺陷"的项：
- 异步子代理并发上限按 status=="running" 计数，interrupt 立即置 cancelled 后线程仍可运行 → start/interrupt 循环可突破 8；线程 panic 后永久 running/泄漏；interrupt 与 completion 竞态可能 Ok 后又被写成 done/failed；children/AGENTS 只增不减。
- request_id 缺失时 gate 生成 A、bootstrap/updates 再生成 B；gateway 自生成 UUID 与 HTTP/审计不一致；`/v1` 未纳入 426 覆盖；审计 hash 只覆盖 prev+action+target+detail，request_id 未纳入且文档未澄清。
- 生成物 writer SSOT 冲突：docs/24 §3 说 docs/generated/*.md 归 `zk docs`，docs/generated/README 说 matrix 归 scripts，CI 无 matrix `--check`。

## 2. 目标

- 异步子代理：并发按**存活线程**计数；线程 panic 通过 catch_unwind/结果通道标记 failed 并释放槽位；interrupt/completion 用原子状态机（cancelled 优先）；完成项有界回收（保留最近 N 或读取后标记）；补压力/竞态测试。
- request_id：最外层中间件为每个请求生成/读取一次并放入 task-local + 响应头，gate/bootstrap/updates/gateway 全部复用；gateway 请求（enterprise native）发送 `X-ZeoKeel-Client`，`/v1/*` 纳入 426（登录/稳定面仍豁免）；审计 hash 语义在 docs/21 澄清（request_id 为运营字段，不参与完整性链）或版本化纳入。
- 生成物 owner：docs/24 §3 writer 表统一 matrix 归属 + `scripts/agent-tool-matrix.py --check` 进入 `scripts/check.sh`/CI；docs/generated/README 更新。
- 全量门禁 + C19 APPROVED。

## 3. 非目标
- `zk api` 路由真源、M6 Docker/多平台。
