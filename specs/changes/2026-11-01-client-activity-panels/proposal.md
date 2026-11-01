# 2026-11-01-client-activity-panels · 提案

- 状态：`done`
- 2026-11-01 修订：**D3 plan 展示被 `2026-11-01-client-plan-mode` 取代**（plan tab order 50/PlanCard/plan.get/update/exit_plan_mode 接管；activity 不展示 plan、不新增 plan 分区，tab order 40 不变）；**goal 改为只读 `goal.get`**（复用 command-surface IPC；`.zeokeel/goal.json` 单 owner；activity.snapshot 不再包含 goal，不新增写面）；turnTail 仅 present deliverables。
- 修订预算：**≤15 文件**（沿用 Lead 例外；移除 plan 展示后不新增文件）
- Owner：dsh-ref
- 日期：2026-11-01
- 关联：Phase 2b 子 spec（用户 2026-11-01 批准 Phase 2 A 方案；与 `2026-11-01-client-composer-surface`、`2026-11-01-client-side-panels` 同批评审）；`research/client-dsh-parity-2026-11-01.md` §3（jobs/subagent/goal/todo/plan/present/schedule）、§4/P1-3/P1-4；`target/audits/zk-client-inventory.md` §4（A3/A4 能力不可见）、§3（B8）、§5（C7）；`target/audits/dsh-reference-inventory.md` §4.3/§4.6（header jobs/subagent、turnTail deliverables/plan/schedule）；`docs/28` §4.6（Jobs/Subagent/Deliverables 后置）、`docs/16`（右栏 tab order）；前置：Phase 1 全部 + Phase 2a/2b
- 评审：docs/reviews/2026-11-01-client-phase2-specs.md
- 变更预算：**15 个文件**（契约 1、生成器源 1（`tools/zk/src/main.rs` 生成 facade）、desktop 1、ui-kit 2、ui-inspector 1、ui-chat 1、desktop-web 3、e2e 2（`e2e/interaction/activity.spec.ts` + `e2e/serve-m1.sh` 剧本科）、文档 3（`docs/16`、`docs/26` §5、`KNOWN-LIMITATIONS.md`）；不含本 spec 目录与生成物）；Lead 批准活动预算例外：14 + docs/26 = 15

## 1. 背景 / 问题

Phase 2a/2b 补齐 composer 与右栏文件/终端/会话操作后，**运行时产物仍没有可见面**：

- 编排/后台能力已有工具实现（`docs/generated/AGENT-TOOL-MATRIX.md`）：`job_list/job_output/job_kill`（`crates/zk-kit-jobs` 进程内 registry）、
  `subagent*`/`agent_team`（`crates/zk-kit-subagent` + `packages/client/tool-agent-team`）、`goal`（`packages/client/tool-goal`，
  `.zeokeel/goal.json` 持久化）、`todo`（`packages/client/tool-todo`，当前进程内 state）、`schedule`（`packages/client/tool-schedule`，
  `.zeokeel/schedules.json`）、`present`（`packages/client/tool-present`）。但 UI 中 `goal/ralph/workflow/subagent/agent_team/job/todo`
  grep 命中 0（`target/audits/zk-client-inventory.md` §4/A3）；用户只能从模型自述或通用工具行间接感知。
- 右栏只有 files/changes/terminal 三 tab（Phase 2b）；没有 activity tab；每轮末尾（turnTail）没有交付物卡（plan 由 plan-mode 接管），
  `present` 工具结果只是一个通用工具行；schedule 任务只有设置里的全局开关/扫描间隔（`ui-settings:377-394`），
  没有任务列表/编辑/暂停/删除（审计 A4）；todo/goal 没有 dock/条。
- 不做的后果：Phase 2「运行时能力可见面」仍缺**编排/后台产物**这一大块；用户无法查看/打断后台任务、无法确认目标进度、
  看不到交付物、无法管理定时任务；能力存在但不可见 = 产品上等于没有。

## 2. 目标

- **G1 右栏 activity tab（order 40）**：在 Phase 2b 右栏新增第 4 个 tab「活动」，聚合展示本会话/本 workspace 的：
  jobs（列表/输出/停止）、subagent 与 agent-team（状态/取消）、goal（状态）、todo（清单）、schedule（列表/暂停/删除/编辑）、
  present 交付物（列表/在右栏预览）；无新 slot（tab 在 `ui-inspector` 组件内，顺序登记 docs/16）；空/加载/错误/离线/六态。
- **G2 jobs 面板**：列出后台 jobs（id/命令/cwd/状态/启动时间/输出摘要），可查看输出与停止；数据来自 `zk-kit-jobs`（进程内 registry）；
  只展示当前 workspace cwd 下的 jobs（其它 workspace 不混入）；停止/输出失败有可读错误。
- **G3 subagent / agent-team 面板**：列出当前会话的 subagent roster（id/depth/status/prompt）与最近一次 `agent_team` 运行结果
  （workers/status，只读），可中断运行中的 subagent；不新增子代理会话/续聊/任务板状态机（复用 `zk-kit-subagent` 与工具行结果）。
- **G4 goal 状态（2026-11-01 修订）**：activity goal 分区只读 `goal.get`（复用 command-surface host IPC；`.zeokeel/goal.json` 单 owner）展示 objective/status/rounds/max_rounds/reason；无 goal 空态；**不再从工具行派生、不新增写面、不产生工具行**。
- **G5 todo 清单**：从当前会话已持久化的 `todo` 工具行（args/result，approval spec 后落库）**按工具语义 replay** 出清单
  （pending/in_progress/completed）；刷新/重进会话可见；不新增第二套 todo 状态/存储（明确「由工具行推导」的边界）。
- **G6 schedule 管理**：列出当前 workspace 的 `.zeokeel/schedules.json` 任务（id/prompt/cron/interval/enabled/next_run），
  支持暂停/启用、编辑 prompt/interval、删除；复用 `zk-tool-schedule` 的公开 Tool 调用（list/update/delete，含 cron 校验与文件形状），
  **不新建第二套文件解析/校验**；不提供新建任务（由 agent 的 schedule 工具创建）。
- **G7 turnTail（每轮尾部，2026-11-01 修订）**：assistant 轮结束后在消息尾部显示：
  - `present` 交付物卡（本轮 `present` 工具调用 → 文件名/描述；点击在右栏 files 预览中打开，复用 Phase 2b 的 file_read/预览）；
  - **仅 present deliverables；plan 由 `client-plan-mode` 的 PlanCard 接管（activity/turnTail 不再展示/折叠计划）**；
  - 无 present 时不显示卡（不造空卡）。
- **G8 复用与边界（C17/C2）**：不新增第二套状态机；jobs/subagent 来自 kit registry，goal 来自 `goal.get`（command-surface 单 owner）；todo/present 来自已持久化工具行，
  schedule 来自 tool-schedule 导出；不新增 UI slot/包/运行期加载；不改变 agent/审批/权限语义；所有动作用户显式触发。

## 3. 非目标

- 不做 plan mode（已由 `client-plan-mode` 接管）；不做 workflow/ralph/goal round driver 的新 UI；activity 不展示 plan、不新增 plan 分区。
- 不做 subagent 会话窗口/续聊/fork、不做 agent-team 任务板（只展示 roster + 最近运行结果）。
- 不做 jobs 新建（jobs 只能由 agent 的 bash `run_in_background` 创建）；不做 job 输出流式跟随（轮询/手动刷新）。
- 不做 schedule 新建/执行历史/多 workspace 管理；不改变 `.zeokeel/schedules.json` 文件形状/cron 语义。
- 不做 todo 写操作/拖拽排序（只读展示）；不修改 `tool-todo` 的进程内状态模型（D2-A）。
- 不做 activity 数据持久化/历史归档；不新增事件通道（轮询，D4-A）。
- 不新增右栏多 pane/float/fullscreen、浏览器 tab、通知/托盘（Phase 3）。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 个人用户 | agent 启动了一个后台任务 | activity tab 看到 job/命令/状态/输出，可停止 |
| 个人用户 | agent 委派了子代理/团队 | 看到 roster/workers 状态，可中断仍在运行的子代理 |
| 个人用户 | 设了目标/待办 | activity goal 只读当前 `.zeokeel/goal.json`（由 `/goal`/GoalBar 写）；todo 清单；刷新后一致 |
| 个人用户 | 建了定时任务 | 列表可见，可暂停/启用/改提示词或间隔/删除 |
| 个人用户 | 本轮有交付文件 | 消息尾部出现 present 交付卡，点击在右栏预览；plan 由 plan-mode 展示 |
| 企业用户 | 同一界面 | 一致；无新权限/网关语义 |

## 5. 成功指标

- activity tab 打开/刷新/2s 轮询/动作后刷新；六态不白屏；tab order 40 与 docs/16 一致；无新 slot/包。
- jobs：列表/输出/停止 e2e（真实 rig 后台 bash）+ host 单测（workspace cwd 过滤、未知 id 错误）。
- subagent/agent-team：roster 状态与 interrupt e2e；agent-team 最近结果只读展示；未知/已结束 interrupt 可读错误。
- goal：只读 `goal.get` 的成功/空/损坏降级测试；**不再从工具行派生**。todo：工具行 replay 单测。
- schedule：list/暂停/启用/编辑/删除 e2e + 文件内容断言；非法 cron/interval 复用 tool 校验；未知 id / prompt>32 KiB 由 host 预校验后拒绝。
- turnTail：present 卡 e2e（点击打开右栏预览）；**不再识别/折叠计划**；无 present 不显示空卡。
- i18n/a11y：`bash scripts/i18n-check.sh` exit 0；axe 0 serious/critical；键盘可达 tab/动作/删除确认。
- 门禁：`cargo test -p zk-contract-ipc -p zk-desktop`、`cargo test --workspace export_schema` + `schema-check`、`zk compose --check`、`zk capability --check`、
  `pnpm -r typecheck/lint/test:unit/build`、`e2e/interaction/activity.spec.ts` + `pnpm e2e:ui`（工具由 `ui-interaction-verification` 落地，当前 UNVERIFIED）全绿；无新增 capability/INDEX 条目。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解/验证 |
| --- | --- | --- |
| jobs/subagent 是进程内全局状态，重启即失；面板不持久 | 中 | 明确只读当前进程；空态/重启说明；KNOWN |
| todo 由工具行 replay，可能与进程内全局 todo 不完全一致/legacy 行缺失 | 中 | 标注「由会话工具记录推导」；损坏/legacy 跳过并提示；D2-A 确认口径 |
| goal 与 `/goal` 真实状态不一致 | 中 | 改为只读 `goal.get`（同 `.zeokeel/goal.json`）；工具行派生删除；空/损坏测试 |
| schedule 直接复用 tool-schedule 调用会写 workspace 文件 | 中 | 仅当前 workspace；复用 tool 的 cron/interval/文件形状校验；**prompt≤32 KiB 由 host 预校验**（tool 本身无长度校验，P2-13）；错误可读；不新增写面 |
| activity 轮询带来 IPC 负载 | 低 | 仅打开 tab 时 2s；可手动刷新；快照有界（jobs≤50、subagents≤64、todo/schedules≤100、输出摘要≤4KiB） |
| turnTail 与 plan-mode PlanCard 重复 | 低 | activity/turnTail 不展示 plan，统一由 plan-mode PlanCard；两处不重复实现 |
| e2e 依赖 fake upstream 剧本科（新增 prompt 分支） | 中 | `e2e/serve-m1.sh` 增加 activity 剧本（job/subagent/todo/schedule/present；goal 用 goal.get/GoalBar 场景）；真实 serve+SQLite；不影响既有用例 |
| 与 side-panels 共享文件（ui-inspector/App/i18n/ipc.rs）竞写 | 中 | 严格顺序：side-panels 合并后本 spec 才开工；共享文件 owner 见 tasks §0b/§5 |

## 7. 已拍板决策（Lead 2026-11-01，按推荐项定稿）

- [x] **D1 schedule 读写路径与面板数据源**：**推荐 A**：新增两个 IPC（`activity.snapshot` 读聚合、`activity.action` 写动作），schedule 动作复用
      `zk-tool-schedule` 的公开 `Tool::call_json`（list/update/delete，含校验），不新建第二套文件解析；备选 B：host 直接读写 `.zeokeel/schedules.json`（重复校验，不推荐）。
- [x] **D2 todo/agent-team 数据口径**：**推荐 A**：todo 从**已持久化工具行** replay；agent-team 只展示最近 `agent_team` 工具行结果；
      不改 `tool-todo`/`tool-agent-team` 的状态模型（不新增第二套状态机）；备选 B：改 tool-todo 为 per-conversation 持久化 + 新读 API（改工具行为/超预算）。
- [x] **D3 plan 展示（已废弃/被取代）**：原 A 仅 Markdown 标题折叠；**2026-11-01 起由 `2026-11-01-client-plan-mode` 接管**（plan tab 50/PlanCard/plan.get/update/exit_plan_mode）。activity/turnTail 删除 plan，不新增 plan 分区；tab order 40 不变。
- [x] **D4 刷新策略**：**推荐 A**：activity tab 打开时 2s 轮询 + 手动刷新 + 动作后刷新，关闭即停（无新事件通道）；
      备选 B：新增 activity 事件推送（需新事件类型/通道，超本 spec）。

> D1/D2/D4 仍有效（snapshot/action；todo/team 工具行 replay；2s 轮询+刷新）；**D3 已被 plan-mode 取代**；goal 改只读 `goal.get`，不进 snapshot。
> 状态保持 `draft` 直至独立评审通过（C19）。

## 8. 变更预算与拆分

- 预计改动文件（实现+文档，不含本 spec 目录/生成物）：
  契约 1（`packages/contracts/ipc/src/lib.rs`：`activity.snapshot`/`activity.action` + payload/results）；
  生成器源 1（`tools/zk/src/main.rs` 生成 `zk-composed` facade 暴露 `zk-kit-jobs`/`zk-kit-subagent`/`zk-tool-schedule`；**不手改 @generated 模板/产物**）；
  desktop 1（`apps/desktop/src/ipc.rs`：snapshot/action + goal.get 读取；无 goal 工具行派生）；ui-kit 2（`types.ts`、`context.tsx`）；
  ui-inspector 1（`index.tsx`：activity tab + 面板内联）；ui-chat 1（`AssistantThread.tsx`：turnTail 仅 present）；
  desktop-web 3（`App.tsx`、`protocol.ts`、`i18n.tsx`）；e2e 2（`e2e/interaction/activity.spec.ts`、`e2e/serve-m1.sh` 剧本科）；
  文档 3（`docs/16` tab order 40、`docs/26` §5、`KNOWN-LIMITATIONS.md`）。**合计 15**（Lead 批准例外：14 + docs/26）。
- 生成物：schema/TS 与 `apps/desktop/Cargo.toml` 由生成器写（C11，不手改）。
- 不拆分：15 ≤ 预算；修订后能力单一（activity 可见面，plan 已移交），复用 kit/tool 行/tool-schedule/goal.get，无新 slot/包/状态机。

## 9. 复用与已有实现（C17）

- 已搜索：`rg -n "job_list|job_kill|subagent|agent_team|goal|todo|schedule|present|turnTail|activity" packages apps crates docs`；
  `./target/debug/zk find kit.jobs` → `crates/zk-kit-jobs/src/lib.rs`；`zk find kit.subagent` → `crates/zk-kit-subagent/src/lib.rs`；
  capability 索引 `kit.jobs`（`shared/contracts/capability-index.json:107`）、`kit.subagent`、`contract.ipc`（`:43`）、`kit.tools`（`:131`）。
- 可复用：
  - jobs：`zk_kit_jobs::{list,output,kill}`（`crates/zk-kit-jobs/src/lib.rs:142-175`；`snapshot` 已含 id/command/cwd/status/stdout/stderr）。
  - subagent/agent-team：`zk_kit_subagent::{list, runner}` + `SubagentRunner::{status,result,interrupt}`（`crates/zk-kit-subagent/src/lib.rs:28-90,123-208`）；
    `agent_team` 工具结果 JSON（`packages/client/tool-agent-team/src/lib.rs:366-379`）。
  - goal：**只读 goal.get**（command-surface；同一 `.zeokeel/goal.json`；不进 snapshot/不派生工具行）。todo/present：**已持久化工具行**（`role="tool"` 行含 name/args/status/result/turn_id）；不新增工具状态 API。
  - schedule：**host 经 `zk-composed` 生成 facade 复用 `zk_tool_schedule` 公开 `Tool::call_json`**（list/update/delete；cron/interval/文件形状校验，`packages/client/tool-schedule/src/lib.rs:76-160`）；
    **前端 app 不 import/不直接依赖 tool 实现**（C4/C2）；host 组合根依赖由生成 facade 隔离。
  - 右栏/预览/turnTail 容器：Phase 2b `ui-inspector`（tab order 40 预留、file_read 预览）与 `AssistantThread` 消息部件；复用其六态/键盘/i18n。
  - i18n/axe：现有 `useUi().t`、`SlotErrorBoundary`、e2e AxeBuilder。
- 本次新增可复用能力：**无新 kit capability/INDEX 条目**；仅 `contract.ipc` 既有契约新增命令 + `zk-tool-schedule` 公开 API 复用（不新建依赖包）。
- 为什么不能复用/扩展：jobs/subagent 无 UI；goal 已有 command-surface `goal.get`/GoalBar（不重复实现）；todo/present 只有工具行（通用卡），需要在右栏/turnTail 形成可读面；
  schedule 只有全局开关，需要列表/编辑面。所有数据源均复用现有 kit/工具行/工具包，不建第二套存储/状态机（C17）。
