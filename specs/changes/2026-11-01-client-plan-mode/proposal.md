# 2026-11-01-client-plan-mode · 提案（Batch2-B：/plan）

- 状态：`approved`
- 评审：docs/reviews/2026-11-01-client-command-batch2.md
- Owner：dsh-ref
- 日期：2026-11-01
- 关联：原父 `client-command-backends` 已按 Lead D7 拆分并删除；本 spec 为 plan-mode 子 spec；DSH §5 `/plan`/`exit_plan_mode`；`docs/19` §2.1b/§11、`docs/27`、`docs/11` §8.6；前置 `client-command-surface`/`client-activity-panels`
- 评审：n/a（新增；实现前 C19/C20）
- 预算：**16 文件**（见 §8；Lead 2026-11-01 书面批准 16 文件例外）

## 1. 背景
`/plan` 当前是 Batch2 禁用占位；无 plan mode、无 `exit_plan_mode`、无右栏 review。用户要求 DSH 1:1：plan 模式只产出 Markdown 计划、不执行 mutating/delegation 工具，提交后本地审核。

## 2. 目标
- `/plan`/`/plan off` 真实命令；plan mode 持久化/恢复。
- plan mode 禁 mutating、允许只读；`exit_plan_mode` 唯一提交；右栏 plan tab order 50 + turnTail 卡；本地批准/拒绝/继续编辑；批准不自动执行。
- 不做 sandbox/企业策略；C20 文件；错误码 `plan/*`。

## 3. 非目标
- 不做 `/compact`/`/skill`（另子 spec）；不做自动执行/远程审批/多人协作；不改企业审批 gate owner；不新增 DSH sandbox。
- 不建第二套计划状态机/Dialog/tab 持久化（复用 side-panels）。

## 4. 用户/场景
| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 计划型个人用户 | `/plan` 描述任务 | 只读调研+Markdown 计划，无写操作 |
| 企业员工 | 计划审核 | 本地批准/拒绝；无企业策略/沙箱 |
| 键盘/读屏 | 操作 plan tab/turnTail | 键盘可达、状态可读、焦点回 composer |

## 5. 成功指标
- 进入 plan mode 后 mutating 工具 0 次执行；只读可用；exit 提交计划。
- 右栏/turnTail 计划可见；批准/拒绝/继续/刷新恢复；`/plan off` 正常。
- 错误码/六态/i18n/axe；P0 测试+证据。

## 6. 风险
| 风险 | 缓解 |
| --- | --- |
| 只靠提示词导致 mutating 逃逸 | agent-rig dynamic_tools 过滤 + host 测试 0 次 |
| 计划状态丢失/审核串会话 | `.zeokeel/plans.json` keyed conversation + 恢复测试 |
| 与 activity/turnTail 冲突 | plan tab order 50；复用 Markdown/持久化 |
| 16 文件预算 | 本 spec 只做 plan；共享文件先写者负责 |

## 7. 已定决策（Lead 2026-11-01，D3/D4/D6 推荐 A/A/B）
- [x] **D3 plan 只读边界/审批**（已定 A）：plan mode 允许只读+exit_plan_mode；禁止所有 mutating **及所有 delegation 工具（subagent/subagent_start/subagent_fork/agent_team/workflow/ralph，即使 mutating=false）**+schedule/MCP 执行；不传播 plan 到 child（后续）；`exit_plan_mode` 唯一提交；提交 `pending_review`；本地批准/拒绝/继续编辑；批准只退出 plan mode，不自动执行；不做 sandbox/企业策略。
- [x] **D4 review/持久化**（已定 A）：右栏 `plan` tab order 50 + turnTail 计划卡；`.zeokeel/plans.json` keyed conversation；刷新/切会话恢复。
- [x] **D6 自动执行/压缩**（已定 B）：不做自动执行/自动压缩；计划批准后由用户下一 turn 触发。
- [x] **拆分/串行**：本 spec 为 plan-mode 子 spec；compact 先实现，本 spec 随后 rebase 共享文件；skill-commands 再后。

## 8. 预算与拆分（16）
1 contract ipc；2–6 `packages/client/tool-plan/{zk.toml,Cargo.toml,src/lib.rs,tests/conformance.rs,README.md}`；7 agent-rig；8 generator `tools/zk/src/main.rs`；9 host `apps/desktop/src/ipc.rs`（plan dispatch/文件/审核）；10 ui-chat `PlanCard.tsx`；11 ui-inspector index（plan tab）；12 ui-kit context（DTO/actions；类型并入）；13 App；14 controller；15 i18n；16 e2e interaction `command-plan.spec.ts`。共享 docs/19、docs/26、KNOWN 由 compact 首写、本 spec 追加（不重复计实现）；`protocol` 解析并入 App。
- 生成物由生成器写；新 tool package 四件套；不新增 capability？`tool.plan` role 需登记能力索引（C17/25）。
- 若超 16：拆 review UI 或工具包，另 spec；**Lead 2026-11-01 书面批准 16 文件预算例外**（单一 plan-mode 目标，16 文件可审计）。

## 9. 复用
- Batch1 菜单/catalog；side-panels plan tab/宽度/持久化/六态；activity turnTail；agent-rig dynamic_tools；`zk-composed` facade（Batch1 set_approval_policy 模式）；Markdown 渲染。
- 新增 `tool-plan` 包（exit_plan_mode）+ agent-rig plan filter；不新建第二套计划状态/审批。
- 不能复用：无既有 plan mode/tool；activity turnTail 只展示标题，不审核；右栏无 plan tab。

## 10. 文档
D3/D4/D6 已定；回写 docs/19（plan mode/审核）、docs/26（IPC/plan 文件/错误码）、KNOWN（无 sandbox/不自动执行）。
