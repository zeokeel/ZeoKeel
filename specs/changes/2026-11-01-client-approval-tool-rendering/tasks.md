# 2026-11-01-client-approval-tool-rendering · 任务清单

> 每个任务必须能独立提交、独立验证；没有 `verify` 的任务不算任务。

## 1. 任务表

| # | 任务 | 写作用域（文件/目录） | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | 契约：`AgentEvent::ApprovalRequested` 扩展元数据 + `ToolCallFinished.args/status` + `ApprovalDecision.allow_session` + `ApprovalGate::decide_scoped`（默认实现）；serde 兼容测试 | `packages/contracts/agent/src/lib.rs`、`packages/contracts/tool/src/lib.rs`、`packages/contracts/agent/tests/*` | — | `cargo test -p zk-contract-agent -p zk-contract-tool` | ☐ |
| 2 | agent-rig：审批请求构造元数据（summary/cwd/read_paths/write_paths/risk/deadline/diff）；按会话记忆 always-allow；emit `ApprovalRequested` 新字段 | `packages/client/agent-rig/src/lib.rs`、`packages/client/agent-rig/src/hooks.rs` | 1 | `cargo test -p zk-agent-rig` | ☐ |
| 3 | 宿主：审批 broker 支持枚举决策 + 超时事件 + 会话级记忆（内存）；`BridgeSink` 写 `{name,args,result,status}`；IPC `approval.decide` 接受新决策 | `apps/desktop/src/approval.rs`、`apps/desktop/src/events.rs`、`apps/desktop/src/ipc.rs` | 1 | `cargo test -p zk-desktop` | ☐ |
| 4 | ui-kit 类型：`AgentEvent`/`ApprovalView`/`ToolCallView` 与解析 DTO 同步（C10 手写镜像的 M1 过渡，沿用 ADR-059 口径） | `packages/ui/ui-kit/web/src/types.ts` | 1 | `pnpm --dir packages/ui/ui-kit test:unit` | ☐ |
| 5 | ui-chat：审批卡全字段/倒计时/默认焦点拒绝/Esc/始终允许/风险 chip/diff 展开 | `packages/ui/ui-chat/web/src/ApprovalPart.tsx`、`packages/ui/ui-chat/web/src/AssistantThread.tsx`、`packages/ui/ui-chat/tests/*` | 4 | `pnpm --dir packages/ui/ui-chat test:unit` | ☐ |
| 6 | 工具行重放：`role="tool"` JSON 解析器 + 归并到 assistant 轮次 + 损坏降级 + 状态五态 | `packages/ui/ui-chat/web/src/{ToolCallPart.tsx,index.tsx}`、`packages/ui/ui-chat/web/src/toolRows.ts`（新）、测试 | 4 | `pnpm --dir packages/ui/ui-chat test:unit` | ☐ |
| 7 | desktop web：`protocol.ts` 解析新事件/usage、`reducer.ts` 工具行/approval 归并、`controller.ts` decide 四值 | `apps/desktop/web/src/{protocol.ts,state/reducer.ts,state/controller.ts}` | 4,5,6 | `pnpm --dir apps/desktop/web test:unit` | ☐ |
| 8 | i18n：`main.tsx` 启动错误边界文案进字典（zh/en），bootstrap 使用默认 locale | `apps/desktop/web/src/{main.tsx,i18n.tsx}`、i18n 资源 | — | `bash scripts/i18n-check.sh` | ☐ |
| 9 | e2e：审批安全字段/默认焦点/超时/始终允许/刷新恢复/损坏行降级/usage 实时 | `e2e/*.spec.ts` | 5,6,7 | `pnpm e2e --grep "approval|tool"` | ☐ |
| 10 | 文档：`docs/19` §4 状态更新、`docs/28` 工具/审批口径、`research/client-dsh-parity-2026-11-01.md` 缺口状态回写 | `docs/19-client-ux-flows.md`、`docs/28-ui-module-map-and-skeleton.md`、research 报告 | 1–9 | `python3 scripts/docs-check.py` | ☐ |
| 11 | 收尾：acceptance 证据、生成物 `--check` | `specs/changes/2026-11-01-client-approval-tool-rendering/acceptance.md` | 1–10 | `zk compose ... --check && zk capability --check` | ☐ |

## 2. 写作用域

- 本变更只写：上表列出的 contracts/agent、contracts/tool、client/agent-rig、apps/desktop、ui-kit、ui-chat、desktop web、e2e、docs。
- 明确不写：`packages/ui/ui-settings*`（属 spec settings-ia）、server/admin、生成物（由生成器写）。
- 并行任务是否重叠：任务 1 先行（2/3/4 依赖）；任务 5/6/7/8 写不同文件可并行；任务 9 依赖 5/6/7。

## 3. 提交顺序

1. 任务 1（契约）
2. 任务 2、3（运行时/宿主，可并行）
3. 任务 4（前端 DTO）
4. 任务 5、6（ui-chat 渲染，可并行）
5. 任务 7、8（app 接线/i18n）
6. 任务 9、10、11（测试/文档/证据）

## 4. 验收前检查

- [ ] `python3 scripts/spec-check.py`
- [ ] `python3 scripts/docs-check.py`
- [ ] 变更涉及的 T0/T1 命令全绿
- [ ] 无手改 `@generated`
- [ ] 复用检查已做（`zk find`/能力索引；新增能力已登记 `@capability`）
- [ ] PR 只包含本 spec 范围
- [ ] `acceptance.md` 已填证据路径
