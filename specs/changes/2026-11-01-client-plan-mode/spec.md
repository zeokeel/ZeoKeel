# 2026-11-01-client-plan-mode · 行为规格

> 状态 draft；D3/D4/D6 已定 Lead 2026-11-01。

## 1. 需求
### R1 · 命令/模式
- **WHEN** 用户输入 `/plan` 或 `/plan off`
- **THEN** 进入/退出 plan mode；菜单「添加」组显示真实 `/plan`（非未实现）；状态持久化 `.zeokeel/plans.json`（conversation keyed），刷新/切会话恢复；命令不发送为普通消息；流式中可进入但提交/审核在非流式执行。
- **AND** 通过 Batch1 唯一 catalog/runCommand；移除 `command/unimplemented` 对 plan。

### R2 · plan mode 行为
- **WHEN** plan mode active
- **THEN** plan mode 工具白名单：**允许 = 只读工具 + `exit_plan_mode`**；**禁止 = 所有 mutating 工具 + 所有 delegation 工具（`subagent`/`subagent_start`/`subagent_fork`/`agent_team`/`workflow`/`ralph`，即使其 `ToolSpec.mutating=false`）+ schedule/MCP 执行**；拒绝时不执行并给可读提示；不依赖提示词，由 agent-rig `dynamic_tools` 过滤保证。
- **AND** **不实现 plan 传播到 child agent**（后续可选）；因此 delegation 工具必须在父层直接阻断，防止子 agent 继承全部 tools 绕过；逐 delegation 工具测试「child mutating 0」。
- **AND** `exit_plan_mode` 工具为唯一提交入口；提交后状态 `pending_review`，计划写入 `.zeokeel/plans.json`；不自动执行计划/不自动退出 plan mode；计划 ≤64 KiB，超限拒绝。
- **AND** 非 plan mode 调 exit_plan_mode → `plan/not-active`。

### R3 · right review/turnTail
- **WHEN** 计划 pending_review
- **THEN** 右栏 `plan` tab（order 50，files/changes/terminal/activity 之后）+ turnTail 计划卡显示计划 Markdown/状态/反馈；**批准**→status approved + mode off（不自动执行），**拒绝**→status rejected + feedback（保持 plan），**继续编辑**→回 composer/plan mode；本地审核，无企业策略/审计/sandbox。
- **AND** 刷新/重启/切会话恢复；`/plan off` 取消草稿（未提交）；审核未知/不存在可读错误。
- **AND** 计划 tab 复用 side-panels 宽度/折叠/持久化/六态；键盘可达、焦点回 composer/计划卡。

## 2. 边界/错误
| 场景 | 期望 | 错误码 |
| --- | --- | --- |
| 非 plan 调 exit | 拒绝 | `plan/not-active` |
| mutating 工具 | 拒绝不执行 | `plan/mutating-blocked` |
| 计划 >64 KiB/写失败 | 拒绝/保留旧 | `plan/too-large`、`plan/failed` |
| 审核未知 | 可读 | `plan/not-found` |
| 流式中提交/审核 | 禁用/提示 | `plan/busy` |
| 跨 workspace/不存在 | 拒绝 | 既有 session/* |

## 3. 权限
- personal/enterprise 本地审核；不接企业策略/sandbox（C6）；不改 `ApprovalPolicy::should_ask`/gate owner；只当前 workspace/conversation。

## 4. 数据兼容
- `.zeokeel/plans.json` schema `{schema:1,conversations:{id:{mode,status,plan,feedback,through_turn,updated_ms}}}`；损坏降级 off；无 DB migration；无 plan 时行为不变；旧客户端不调用新 IPC。
- 回滚删除文件/回退 commit；计划仅本地。

## 5. 验收断言
- [ ] `/plan`/`off` 真实命令、状态持久化；Batch1 不回归。
- [ ] plan mode mutating 0 次、只读可用；exit_plan_mode 提交；非 plan 拒绝。
- [ ] 右栏 plan tab/turnTail 审核批准/拒绝/继续；刷新/切会话恢复；批准不自动执行。
- [ ] 64 KiB/错误/六态/键盘/axe/i18n；错误码登记；C20 P0 证据。
- [ ] 无 sandbox/企业策略；未跑用户测试 UNVERIFIED。

## 6. 未决
- [x] D3/D4/D6 已定；实现细节 design/tasks。
