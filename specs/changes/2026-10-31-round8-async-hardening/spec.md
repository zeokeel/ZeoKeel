# 2026-10-31-round8-async-hardening · 规格

- 状态：`approved`

## 1. 异步子代理
- `RigSubagentRunner::start` 用 `Arc`/`AtomicUsize` live_threads 计数，线程入口减计数（RAII guard）；达上限拒绝。
- 线程体包裹 `std::panic::catch_unwind(AssertUnwindSafe(...))`，panic → status failed + 错误结果 + 释放槽位（记录 stderr，不传播）。
- 状态机：`Running -> Done|Failed|Cancelled` 用 Mutex + 原子 compare；interrupt 置 flag 并 CAS Running→Cancelled；完成线程仅 CAS Running→(Done/Failed)，若已 Cancelled 保留 cancelled。取消结果以 `cancelled` 错误返回。
- 保留策略：children map 上限（如 64）；完成超限时移除最旧已完成项（仍可通过 list_agents 看当前 cap 内）；kit AGENTS 同样 bounded（或 finish 标记 + 清理）。
- 测试：start/interrupt 循环后 live 线程 ≤ cap；panic 子代理（测试 runner/mock 触发 panic）→ failed + 槽位释放；interrupt/completion 竞态重复运行状态合法；完成后 map 有界。

## 2. request-id / gateway 426
- 最外层中间件：读取 `X-Request-Id`（合法 ≤128）或生成 UUID，写入 task-local，响应头 echo；所有 handler 用 `current_request_id()`。
- enterprise native gateway：llm-gateway 的 GatewayLlm 请求带 `X-ZeoKeel-Client`（env `ZEOKEEL_CLIENT_HEADER`，desktop boot 设置）。**gateway `/v1/*` 的 426 门禁本轮不启用**：rig-core 原生 Responses 客户端不支持自定义头（上游限制），网关依赖 JWT+发布锁定；docs/04 记录例外，列入后续。
- 审计：docs/21 明确 hash 链覆盖字段（prev/action/target/detail）；`request_id` 为运营字段，不参与 hash；如需纳入走 schema 版本化（本轮只文档化）。

## 3. matrix/writer
- docs/24 §3 writer 表：`docs/generated/AGENT-TOOL-MATRIX.md` 归 `scripts/agent-tool-matrix.py`（或改由 zk docs，二选一并一致）；docs/generated/README 同步；`scripts/check.sh` + CI 增加 `python3 scripts/agent-tool-matrix.py --check`。
