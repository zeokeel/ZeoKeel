# 2026-11-01-client-approval-tool-rendering · 验收与证据

- 验收人：待填
- 日期：待填
- PR/commit：待填

> 状态：`draft`（未开工）。实现完成后按任务表逐项填证据路径。

## 1. 验收命令与结果

| 命令 | 结果 | 证据路径 |
| --- | --- | --- |
| `cargo test -p zk-contract-agent -p zk-contract-tool` | 待跑 | — |
| `cargo test -p zk-agent-rig` | 待跑 | — |
| `cargo test -p zk-desktop` | 待跑 | — |
| `bash scripts/i18n-check.sh` | 待跑 | — |
| `pnpm -r typecheck && pnpm -r lint && pnpm -r test:unit` | 待跑 | — |
| `pnpm e2e`（approval/tool 用例） | 待跑 | — |
| `zk compose --edition personal --profile default --channel official --check` | 待跑 | — |

## 2. 手工检查

- [ ] 审批卡展示 diff/cwd/路径/风险/倒计时
- [ ] 默认焦点拒绝、Esc 拒绝、超时拒绝
- [ ] 始终允许本会话后同工具不再弹窗；新会话恢复询问
- [ ] 刷新后工具卡完整恢复；损坏行走降级
- [ ] usage 实时递增且终态一致

## 3. 文档与决策

- [ ] `docs/19` §4 已更新
- [ ] `docs/28` 工具/审批口径已更新
- [ ] parity 报告缺口状态已回写
- [ ] `docs/README.md`：不适用

## 4. 回滚

- 回滚步骤：回退 commit；旧会话数据兼容
- 数据影响：无迁移
- 已验证回滚：待填

## 5. 剩余风险 / 后续

| 项 | 影响 | 后续 spec |
| --- | --- | --- |
| 系统通知/托盘红点（docs/19 §4.2） | 中 | Phase 3 诊断/通知 |
| 工具卡 keyed view（54 种） | 低 | Phase 2 capability surfaces |
| 结构化 tool_calls 存储 | 低 | Phase 2 评估 |

## 6. 结论

- [ ] `done`：证据齐全，可合并
- [ ] `abandoned`：原因：待填

## 6. 复用证据（C17）

- 搜索记录：`rg "ApprovalGate|approval.decide|parseAgentEvent" packages apps`
- 复用清单：`ApprovalBroker`、`BridgeSink`、`ToolCallPart`、`translate`
- 新增能力登记：`@capability contract.approval`（扩展）
