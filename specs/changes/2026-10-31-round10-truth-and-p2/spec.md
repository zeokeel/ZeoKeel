# 2026-10-31-round10-truth-and-p2 · 规格

- 状态：`approved`

## 1. 文档真源
- docs/03 §6.1 CI 示例删除 `zk api`、`zk verify-lock`（或标 pending）；补“实现状态：未实现，见 docs/24 §3/§4 backlog”。
- AGENTS §6 命令清单：移除不存在命令或明确标注“未实现”；保持可执行命令与实际一致。
- docs/24 §3 表：`zk api`/`zk lock`/`zk metadata snapshot` 行标注“未实现（backlog）”；§4 路由真源时序标注“未实现”。
- docs/generated/README 已有说明，保持。
- docs/19 增加 schedule 设置（开关/扫描间隔/settings 优先/env fallback）；docs/38 async retention 说明。

## 2. request-id
- kit `request_id_from_headers`：仅接受 `[A-Za-z0-9._-]` 且 1..=128；非法/缺失生成 UUID；响应/审计/gate body 一致。测试非法字符/超长/合法透传。
- docs/04 §7 末句改为“HTTP request_id 全链路关联；gateway settlement id 服务端独立生成”。

## 3. async / schedule e2e
- agent-rig：新增 spawn-failure（构造 runner 启动失败）与真实线程并发压力（start/interrupt 循环，live≤cap）测试；docs retention 语义。
- schedule：`e2e/m1-serve.spec.ts` 增加设置页“调度/自动化”开关+间隔保存，reload 后 settings.get/UI 仍为保存值；若 executor 自动 turn 可稳定构造则加一条，否则记录 UNVERIFIED。
