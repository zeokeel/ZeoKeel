# 2026-11-01-client-command-surface · 提案（Batch 1：DSH 1:1 命令面）

- 状态：`done`
- Owner：dsh-ref
- 日期：2026-11-01
- 关联：用户 2026-11-01 新要求（按 DSH 1:1 抄命令面）；`target/audits/dsh-reference-inventory.md` §5；`docs/19` §2/§11、`docs/27` §2.5–§2.8、`docs/11` §8.6；前置：`2026-11-01-client-composer-surface`（`/` 菜单框架、附件、引用、模型只读标签）；`2026-11-01-client-approval-tool-rendering`（approval 交互）；`2026-11-01-client-side-panels`（右栏/会话操作）
- 评审：docs/reviews/2026-11-01-client-command-surface.md
- 变更预算：**24 个文件**（见 §8；Lead 2026-11-01 书面批准 24 文件/约 728 行例外）

## 1. 背景 / 问题

`2026-11-01-client-composer-surface` 已把 composer `/` 菜单接上 app 的唯一命令目录，但该目录当前是 **UI 动作**：`new-conversation`、`toggle-sidebar`、`open-settings`、`toggle-inspector`、`cancel`、`retry`（enterprise 再加 skills/connectors/apps）。用户新要求是**按 DSH 1:1 抄命令面**：composer `/` 改为 DSH 的斜杠命令集（`/file` `/model` `/permission` `/goal` `/export` `/feedback`；Batch 2 的 `/compact` `/plan` `/skill` 占位），UI 动作只留在 ⌘K 命令面板。

DSH 事实（`target/audits/dsh-reference-inventory.md` §5）：`/` 菜单空查询固定两节「添加」= `file`/`goal`/`plan`/`feedback`，「指令」= `compact`/`permission`/`model`/`export`；键位 `/` 打开、键入本地筛选、`↑/↓` 移动、`Enter`/`Tab` 接受、`Esc`/`Shift+Tab` 回 composer；命令解析为 `leadingInput`/`popupSelect`/`action`/`execute` 四类；**未知/非法命令被拒绝，不静默当提示词**；带附件只有声明 `input.attachments` 的命令可接收；DSH 没有全局 ⌘K 执行面（我们保留自己的 ⌘K UI 动作面板，不冒充 DSH）。

不做的后果：`/` 菜单与 DSH 语义不一致，用户按 DSH 肌肉记忆输入 `/model`/`/goal` 会被当普通消息发送或执行错误动作，命令面不可用且会产生错误数据。

## 2. 目标

- composer `/` 菜单 = DSH Batch 1 命令集（6 个可用 + 3 个 Batch 2 占位），与 DSH 的分组/键位/弹窗语义 1:1（不可移植处显式记录）。
- 移除 `/` 中的 UI 动作；这些动作在 ⌘K 中保留且行为不变。
- 未知/非法 `/` 命令**不发送**、给出可读错误并保留草稿；Batch 2 命令显示「未实现」且不可执行，不静默降级。
- `/file`、`/model`、`/permission`、`/goal`、`/export`、`/feedback` 每条有可执行实现与六态/错误/焦点/i18n/a11y。
- `/goal` 在 composer 上方显示 GoalBar，与 DSH 位置一致。
- 会话级 model/permission override 只影响当前会话后续轮次，切会话/重启可恢复，失败回滚。
- 契约/IPC/错误码登记 `docs/26` §5；无新增 capability；不改 workspace 归属（C18）。

## 3. 非目标

- 不做 Batch 2：`/compact`、`/plan`（plan mode/审批/状态机）、`/<skill>` 的实际执行；本 spec 只做**禁用占位**，另 spec 实现。
- 不新增全局命令面板/不改变 ⌘K 的 UI 动作集合与语义（仅把 `/` 与 ⌘K 的目录分流）。
- 不新增 sandbox/沙箱；`/permission` 只切换**审批策略**，不承诺限制员工操作（弱管控，C6）。
- 不做服务端新端点（`/feedback` 企业版服务端提交若无端点则本地并标 UNVERIFIED）。
- 不改 tool-goal/tool-schedule 等工具行为；`/goal` 只复用其公开 driver/存储。
- 不改 workspace/conversation 归属与数据库 schema（会话 override 用 workspace 元数据文件；若后续迁移另 spec）。
- 不做 DSH 的 18 条可配置快捷键/快捷键速查（另 spec）；本 spec 只做命令菜单的固定键位。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 个人版用户 | 在会话中输入 `/` | 看到 DSH 分组命令；`/file` 选附件；`/model` 选本地模型+推理等级；`/permission` 切审批；`/goal` 建目标；`/export` 导出；`/feedback` 反馈 |
| 企业版员工 | 登录网关后输入 `/` | `/model` 只列网关模型；model/permission 仍会话级；`/feedback` 明确本地保存（服务端端点未实现） |
| 高频键盘用户 | 输入 `/mo`、↑↓、Tab/Enter、Esc | 与 DSH 相同的接受/取消/回退；不误发消息 |
| 辅助技术用户 | 读屏/键盘操作命令菜单与弹窗 | role/aria 正确、焦点回 composer；错误可读可恢复 |

## 5. 成功指标

- `/` 菜单空查询分组与 DSH 一致；6 个 Batch1 命令可完成各自动作；3 个 Batch2 占位不可执行且文案标注「未实现」。
- 未知 `/xxx` 0 次被作为普通提示词发送；Batch2 占位 0 次执行；`/` 中 0 个 UI 动作（这些只在 ⌘K）。
- 会话 override 切换/重启恢复；失败回滚；跨会话不串。
- 每条命令有 P0 交互测试 + 证据；axe 0 serious；i18n zh/en 无硬编码。
- `spec-check`/`docs-check`/`cargo`/`pnpm`/交互门禁（工具落地后）全绿。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解/验证 |
| --- | --- | --- |
| DSH 的权限预设含 sandbox（只读/工作区写/完全权限），我们只有审批策略、无沙箱 | 高（语义不等价） | design 明确映射与不可移植差异：只读→Strict（仍不能阻止写，只审批）、工作区写→AutoReadOnly、完全权限→Ask；UI 文案标注「仅审批策略」；不宣称沙箱 |
| 会话级 model 覆盖与 agent 全局 `configure_model/configure_agent` 冲突/竞态 | 高（错用模型/串会话） | host 在每次 turn 前按会话应用 override 并在 turn 后恢复全局；**全局 config-mutex 使其它 conversation 的 turn 排队，`settings.set` 的 apply/publish 同锁**；失败/取消回滚；排队 turn 取消后跳过不执行；测试覆盖 |
| `/export` 大文件/载体差异（Tauri vs serve） | 中 | 大小上限、流式/分块或截断提示；统一 host 生成，前端下载/路径提示；失败可读 |
| `/feedback` 企业版无服务端端点 | 中（假成功） | 默认本地文件 + 明确「本地保存」；服务端目标标 UNVERIFIED，不做假成功 |
| 菜单目录与 ⌘K 共用 catalog 导致回归 | 中 | 单一 catalog + `surfaces` 过滤；单测断言两 surface 集合；旧 ⌘K 快照不变 |
| Batch2 占位被误执行 | 高 | 占位 `disabled` + `unimplemented` 态；执行入口再次守卫；测试断言不发送消息/不调 IPC |
| 审计/安全：export/feedback 含用户数据 | 中 | 不写日志/不跨 workspace；feedback 不含密钥；导出文件在 workspace 内；大小上限 |
| C10 边界校验 | 高 | 新 IPC 全部 Rust DTO + generated schema；前端 Ajv/生成 validator；拒绝未知/超限 |

### DSH 差异（Batch1，诚实记录）

- `/model`：DSH 是 provider/模型/推理等级列表；我们 personal 只有单个 `provider.model`，**没有模型发现数据源** → Batch1 降级为「当前模型 + 手动输入模型名 + recentModels(localStorage≤8) + reasoning effort」，enterprise 用 bootstrap 的 gatewayModels（不新增 provider 发现/不硬编码列表）。
- `/export`：DSH ZIP；Batch1 Markdown（见 D3；ZIP 留 Batch2）。
- `/feedback`：DSH 平台提交/反馈弹窗；Batch1 本地 JSON + 明确「本地保存」，企业服务端无端点 → UNVERIFIED（D4）。
- `/permission`：DSH 含 sandbox 预设；Batch1 仅审批策略 Strict/AutoReadOnly/Ask，无沙箱、不限制员工（D6）。
- `/compact /plan /skill`：Batch2 占位禁用（D5）。

## 7. 已定决策（Lead 2026-11-01 批准推荐 A）

- [x] **D1 会话级 model/permission override 的存储与恢复**：**推荐 A**：host 侧 workspace 元数据文件 `.zeokeel/conversation-overrides.json`（workspace 内文件按 `conversation_id` 键，workspace 由文件位置隐含；原子写、无 DB migration、无密钥；切会话/重启读取并应用；不进全局 settings）。**enterprise gatewayModels 来自 bootstrap，经 `auth.configure` 可选 `models[]` 缓存到 host，`session.override_set` 校验；UI 仅展示**。B：SQLite 新表/列（需要 v7 迁移，与刚完成的 side v6 串行，成本高）；C：仅进程内存（重启丢失，不满足“恢复”）。**已定 A**
- [x] **D2 `/goal` 后端**：**推荐 A**：新增 host IPC `goal.get`/`goal.set`/`goal.update`，host 用 `tool-goal` 公开 driver（`goal/set_goal/update_goal/cancel_goal`）与同一 `.zeokeel/goal.json`，不产生 agent 工具行；GoalBar 读同一状态。B：通过 `goal` agent 工具走 chat（会产生消息/延迟/可能被审批），不推荐。**已定 A**
- [x] **D3 `/export` 格式与保存**：**推荐 A**：Batch1 用 **Markdown**（UTF-8，含正文/工具行摘要/usage；文件名 `<title>-<yyyyMMdd-HHmm>.md`）由 host `session.export` 写 workspace `.zeokeel/exports/` 并返回相对路径，serve 提供下载；Tauri 无 opener → 写文件后展示/复制路径（`system.open_url` 仅 http(s)，不承诺自动打开文件）；上限 8 MiB。B：DSH 同款 ZIP（含 md/jsonl/usage），需要新增下载路由/大文件分块/ZIP 校验，Batch2 再做。**已定 A**（Batch1 可控；ZIP 差异记录 KNOWN-LIMITATIONS，Batch2 补）。
- [x] **D4 `/feedback` 提交目标**：**推荐 A**：个人版与企业版都先写本地 `.zeokeel/feedback/<conversation>-<timestamp>.json`（分类 bug/idea/other + 可选描述 ≤4 KiB + 版本/edition/平台，不含密钥；UI 明示“本地保存”）；企业版服务端端点不存在 → 不做网络提交，标 UNVERIFIED，待服务端 spec。B：新增服务端反馈端点（超出 Batch1 预算，需要鉴权/审计/存储）。**已定 A**
- [x] **D5 Batch2 占位是否显示**：**推荐 A**：显示 `/compact` `/plan` `/skill` 为禁用项并标「未实现（Batch 2）」；`/plan` 归「添加」、`/compact` 归「指令」、`/skill` 单独「技能」组末位；不可执行、不发送。B：隐藏（用户无法预知 Batch2；不推荐）。**已定 A**
- [x] **D6 `/permission` 三档映射**：**推荐 A**：DSH「只读/工作区写/完全权限」→ 我们 `Strict/AutoReadOnly/Ask`（只审批，不沙箱），UI 标注差异与“不能限制员工操作”；B：新增沙箱（拒绝，C6/架构不允许）。**已定 A**

> D1–D6 已由 Lead 于 2026-11-01 按推荐 A 批准；状态仍保持 `draft`，待同一评审员复评通过后进入实现。

## 8. 变更预算与拆分

- 预计改动文件：**24**（实现+文档，不含本 spec 目录/生成物；**Lead 2026-11-01 书面批准 24 文件/约 728 行例外**）：
  - 契约 1：`packages/contracts/ipc/src/lib.rs`（新命令/DTO/错误码 + `auth.configure` 可选 gatewayModels 缓存字段；不改 `ChatSendPayload`/session 表）
  - host 3：`apps/desktop/src/ipc.rs`、`apps/desktop/src/command_surface.rs`（新：goal/override/export/feedback 实现）、`apps/desktop/src/main.rs`（模块声明）
  - 生成器源 1：`tools/zk/src/main.rs`（`zk-composed` generated facade 暴露 `set_approval_policy`，供 host 单点注入；不手改生成物）
  - ui-kit 3：`types.ts`、`context.tsx`、`index.tsx`（**共享 Modal 实现放 index.tsx**，供 ui-sessions/ui-chat 复用；不单独新增 Modal.tsx）
  - ui-chat 5：`AssistantComposer.tsx`、`CommandMenu.tsx`（新）、`GoalBar.tsx`（新）、`FeedbackDialog.tsx`（新）、`tests/command-surface.test.tsx`（新）
  - ui-sessions 1：`index.tsx`（**迁移私有 Modal 到 ui-kit 共享 Modal**，禁止两套 Dialog）
  - desktop-web 4：`App.tsx`、`state/controller.ts`、`protocol.ts`、`i18n.tsx`
  - app 测试 1：`apps/desktop/web/tests/command-surface.test.tsx`（新）
  - e2e 2：`e2e/m1-serve.spec.ts`（追加）、`e2e/interaction/command-surface.spec.ts`（新）
  - 文档 3：`docs/19`、`docs/26`、`KNOWN-LIMITATIONS.md`（docs/16 由统一回写任务处理，不占本预算）
  - 合计 **24**。拆分：本 spec 为单一 DSH 命令面目标，Lead 已书面批准例外；若实现中超 28 文件，先拆 `/export` 子 spec。

- 生成物：`shared/contracts/{schema,ts}`、`apps/desktop/web/src/generated/*` 由生成器写（C11，不手改）。
- 是否拆分：不拆（Batch1 是同一命令面）；Batch2 另 spec（`/compact` `/plan` `/skill` 后端）。

## 9. 复用与已有实现（C17）

- 已搜索：`rg -n "AssistantComposer|buildCommandCatalog|runCommand|composer" packages apps`；`target/audits/dsh-reference-inventory.md` §5；`zk find kit.goal`；`rg -n "configure_model|configure_agent|ApprovalPolicy|goal\\(|set_goal|session_export|feedback" packages apps docs`。
- 可复用：
  - `/` 菜单框架/附件/引用/IME/焦点：`packages/ui/ui-chat/web/src/AssistantComposer.tsx`（composer-surface 已实现）。
  - 唯一 app 命令目录/执行：`apps/desktop/web/src/App.tsx` `buildCommandCatalog`/`runCommand`（扩展 `surfaces` 字段分流 `/` 与 ⌘K，不建第二套）。
  - 附件选择/上传：`workspace.attachment_save` + composer 附件流程（不新实现文件选择/校验）。
  - 模型应用：`zk-contract-agent::Agent::configure_model` / `configure_agent`（`packages/contracts/agent/src/lib.rs:327-340`；`configure_agent` 接受 `AgentSettings` 含 `completion.reasoning_effort`）。
  - 审批策略：`zk_contract_config::ApprovalPolicy`（`packages/contracts/config/src/lib.rs:136-170`；`should_ask` 单一实现）。
  - Goal：`packages/client/tool-goal/src/driver.rs` 的 `goal/set_goal/update_goal/cancel_goal`（同一 `.zeokeel/goal.json`，keyed conversation）与 `tool-goal` 公开 `Tool::call_json`。
  - 持久化/导出来源：`store-session-sqlite` 的会话/消息/usage 读取；`serve`/Tauri transport。
  - 弹窗/按钮/焦点：`@zeokeel/ui-kit` Button + 现有 Modal 模式（`ui-sessions` 私有 Modal 是反例：本 spec 提升为 ui-kit 共享原语，C17）。
- 本次新增可复用能力：
  - `ui-kit` 的 `Modal`/`Dialog` 共享原语（若现有 `ui-sessions` Modal 可提升则复用；能力 id 不新增，登记 ui-kit）。
  - host `command_surface` 的 session override/goal/export/feedback helper（业务编排，非通用库；不登记 capability）。
- 为什么不能复用/扩展：DSH 命令类型（popupSelect/action/execute/leadingInput）与现有 UI 命令 catalog 不同；但可给同一 `CommandDescriptor` 增加 `surface`/`kind`，避免第二套菜单/执行器。`/export` 无现成导出；`/feedback` 无现成端点；`/goal` 工具已有但 UI 需要 host 读（不能从 renderer 直接读文件）。
