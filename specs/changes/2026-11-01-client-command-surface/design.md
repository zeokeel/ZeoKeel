# 2026-11-01-client-command-surface · 设计（Batch 1）

> 实现选型与取舍；行为见 `spec.md`。状态 `draft`，按 proposal §7 **已定（Lead 2026-11-01，D1–D6 推荐 A）** 设计。

## 0. 决策摘要

| 决策 | 选择 | 理由 |
| --- | --- | --- |
| D1 会话 override 存储 | **A：workspace 元数据文件 `.zeokeel/conversation-overrides.json`** | 无 DB migration（side-panels 刚做 v6）；切会话/重启可恢复；不含密钥；原子写 |
| D2 /goal 后端 | **A：host IPC + tool-goal 公开 driver** | 复用同一 `.zeokeel/goal.json`/状态机；无工具行/审批/延迟；C17 |
| D3 /export | **A：Markdown（Batch1）** | 可控大小/载体；DSH ZIP 差异记 KNOWN，Batch2 补 ZIP |
| D4 /feedback | **A：本地文件，企业服务端 UNVERIFIED** | 不假成功；无服务端端点；Batch1 离线可用 |
| D5 Batch2 占位 | **A：显示 disabled + 「未实现（Batch 2）」** | 可发现、不可误执行；不静默降级 |
| D6 /permission 映射 | **A：Strict/AutoReadOnly/Ask（仅审批）** | 无沙箱；不承诺限制员工（C6） |

## 1. 影响面

| 类型 | 变更 |
| --- | --- |
| 包 | `@zeokeel/ui-chat`（命令菜单/GoalBar/FeedbackDialog）、`@zeokeel/ui-kit`（共享 Modal/命令 DTO/actions）、`@zeokeel/ui-sessions`（迁移私有 Modal 到 ui-kit）；**不新增包/slot** |
| 契约 | `zk-contract-ipc` additive：`goal.get`/`goal.set`/`goal.update`、`session.override_get`/`session.override_set`、`session.export`、`feedback.submit` + DTO；`auth.configure` 可选 `models[]` 缓存（enterprise gateway 列表来源）；`CommandDescriptor` 在 ui-kit（非 Rust 契约）加 `surface/kind/disabledReason`；不改 ChatSendPayload |
| 路由 | 无新 HTTP 路由；全部本地 IPC；serve 与 Tauri 同一 `/ipc`；export 由 host 写 workspace 文件 |
| 迁移 | 无 DB migration；新增 workspace 元数据 `.zeokeel/conversation-overrides.json`、`.zeokeel/exports/`、`.zeokeel/feedback/` |
| 生成物 | `shared/contracts/{schema,ts}` 由 `generate_schema` 写；`apps/desktop/web/src/generated/*` 与 `zk-composed` 由 compose 写；生成器源 `tools/zk/src/main.rs` 增加 `set_approval_policy` facade（不手改生成物） |
| 文档 | `docs/19`（命令面 UX）、`docs/26` §5（IPC/错误码/文件）、`KNOWN-LIMITATIONS`（ZIP/服务端 feedback/会话 override 限制）；docs/16 由统一回写任务处理 |
| 兼容 | 旧会话默认全局；旧客户端不调用新命令；⌘K catalog 行为不变 |

## 2. 方案

### 2.1 单一命令目录与两 surface（C17）

`packages/ui/ui-kit/web/src/types.ts` 扩展已有 `CommandDescriptor`（composer-surface 定义）为：

```ts
type CommandSurface = "slash" | "palette";
type CommandKind = "action" | "popupSelect" | "leadingInput" | "execute" | "unimplemented";

interface CommandDescriptor {
  id: string;
  title: string;
  group: string;
  hint?: string;
  disabled?: boolean;
  disabledReason?: string;
  surfaces: ReadonlyArray<CommandSurface>; // UI 动作仅 palette；DSH 命令仅 slash
  kind: CommandKind;
}
```

- `App.tsx` 的 `buildCommandCatalog` 仍为**唯一 catalog**；新增 DSH slash 行（id 如 `slash-file`…）与 Batch2 占位；UI 动作 `surfaces:["palette"]`，DSH 行 `surfaces:["slash"]`。
- `/` 菜单由 `AssistantComposer` 过滤 `surfaces.includes("slash")`；⌘K 过滤 `palette`；两者都调唯一 `runCommand(id)`；不存在第二套执行器。
- 空查询分组顺序由 catalog 的 `group` + 显式 `groupOrder`（添加=0、指令=1、技能=2）；非空查询按 title/别名过滤。
- Batch2：`kind:"unimplemented"`、`disabled:true`、`disabledReason`；执行入口（`runCommand`/菜单 accept）二次守卫，绝不提交消息。

### 2.2 命令菜单交互（DSH 1:1）

- 词首 `/` 触发（`detectComposerTrigger` 复用 composer-surface 实现）；`/` 在正文中间不触发；IME composing 不触发。
- 键位：`↑/↓` 移动、`Enter`/`Tab` 接受、`Esc`/`Shift+Tab` 关闭；菜单内焦点留 textarea，`aria-activedescendant`；点击外部关闭；disabled 项可见不可选。
- `popupSelect`（model/permission）在 composer 上方用同一 `CommandMenu` 的 listbox 渲染，初始高亮=当前值；`leadingInput`（goal）选择后把 `/goal ` 留在输入框并聚焦，等用户输入 objective；`execute`（export）立即执行并显示 toast/内联状态；`action`（file/feedback）打开选择器/弹窗。
- 附件边界：`/file` 是 action；带附件时选其它命令→ `command/attachments-unsupported`，保留 chip/草稿（DSH 同语义）。

### 2.3 会话级 model/permission override（D1 + P1-2/P1-3）

**存储**：`<workspace_root>/.zeokeel/conversation-overrides.json`（host 原子写，schema `{schema:1, conversations:{<conversation_id>:{model?, reasoning_effort?, approval_policy?, updated_ms}}}`）。workspace id 由会话归属校验得到；文件不存 key/URL。损坏/未知字段降级为空并提示（不覆盖用户文件）。

**模型来源（P1-3 诚实降级）**：
- personal：**无 provider 模型发现数据源**；popup = 当前模型 + 手动输入模型名 + `recentModels`（localStorage，最多 8，仅成功应用过的名字；跨 workspace 客户端列表）+ reasoning effort。手动值 trim、≤200 字符、无控制字符；空/非法拒绝。
- enterprise：bootstrap 返回的 `models[]` 由前端在登录后经 `auth.configure` 的 additive `models?: string[]` 回传；host 缓存该列表（登录/刷新更新、登出清空），`session.override_set` 校验模型在列表内；UI 只展示 host/前端缓存的列表，不硬编码、不持有上游 key（C7）。

**IPC**（additive，docs/26 §5）：
- `session.override_get {workspace_id, conversation_id}` → `{model, reasoning_effort, approval_policy, is_session_override, recent_models?}`（recentModels 客户端本地，不必 IPC）。
- `session.override_set {workspace_id, conversation_id, model?, reasoning_effort?, approval_policy?}` → `{applied:true, overrides}`；host 校验归属、枚举范围、空值清除（`null`）、personal/enterprise 模型规则；失败不写盘。

**应用（P1-2：全局 config-mutex + 排队）**：
- desktop 增加单一 **global config mutex**（`tokio::sync::Mutex<()>`，进程内）；所有 conversation 的 turn 在开始前**必须**获取它，持有到 turn 结束/恢复后释放；互斥期内其它 conversation 的 turn 排队（不并发使用全局 agent 配置）。
- 持锁后：读取该 conversation override（无 override 直接用全局 settings，不调用 configure_*）；有 override 时：
  - personal → `configure_model(base_url, api_key, override.model ?? global.model, provider)` + `configure_agent(AgentSettings{completion.reasoning_effort: override.effort ?? global.effort, ..})`；
  - enterprise → 网关凭证 + override.model/global gateway model 调 `configure_model(..., "gateway")` + `configure_agent` 应用 effort；
  - permission → 通过 agent-rig 的 **`set_approval_policy` 单点**注入 per-turn `ApprovalPolicySource`（由 override/global 构造），**不包裹/不替换 ApprovalGate owner**、不改 `should_ask`。
- turn 开始前记录 global settings snapshot/revision；turn 结束恢复全局 settings（revision 未变→snapshot；已变→重新读取最新全局，避免旧代际覆盖）；异常/取消也必须恢复（RAII/guard），失败回滚到上一成功配置并提示。
- **settings.set 纳入同一 mutex**：`SETTINGS_SET` 的 `apply_agent_settings`/provider publish 在保存后也获取 global config-mutex（或排队到当前 turn 结束）再应用；turn 持有 override 期间 settings.set 不得改变当前 turn 的 model/permission；turn 结束后按 revision 决定 snapshot vs 最新全局。补「turn 运行中 settings.set」测试（保存不打断当前 override、结束后新全局在下一 turn 生效）。
- **排队 turn 取消（P2-1）**：`chat.cancel` 只取消当前运行 turn；排队 turn 在获得 mutex 后先检查 cancellation flag（由 cancel 标记），若已取消 → 不执行 agent、不产生事件/消息/错误，释放锁并结束；补排队取消/并发测试。
- 不再声称“现有单 turn 串行”或“跨会话并发 turn 不支持”；明确本 spec 用全局互斥换取正确性，turn 吞吐下降记录 KNOWN/无用户可见超时（排队不丢请求）。
- `modelLabel`/`modelSettingsPane` 保留全局默认展示；`/model` 显示会话覆盖（“会话覆盖：X；默认：Y”）。

### 2.4 /permission（DSH 预设映射）

| DSH 预设 | 我们的 `ApprovalPolicy` | 实际行为 | 不可移植差异 |
| --- | --- | --- | --- |
| 只读 / read-only | `Strict` | 所有工具都询问 | 无 sandbox，不能阻止写；只增加询问 |
| 工作区写 / workspace-write | `AutoReadOnly` | 只读工具静默，变更工具询问 | 不限制路径/网络；无“自动审查”模型 |
| 完全权限 / full-access | `Ask` | 仅 `requires_approval` 工具询问 | 不能绕过 weak invariant（requires_approval 永远询问）；不是沙箱绕过 |

- 会话级应用（已冻结）：host 在 global config mutex 内、turn 前通过 agent-rig 的 **`set_approval_policy(Some(Arc<dyn ApprovalPolicySource>))`** 注入当前会话 policy（override 优先，否则全局）；turn 结束恢复全局/清空。**不包裹 ApprovalGate、不改 gate owner、不新增第二套审批实现**；generated `zk-composed` facade 提供该单点（生成器源 `tools/zk/src/main.rs`）。UI 显示「仅审批策略，不限制工具/网络/文件」。
- 不改全局 `AppSettings.tools.approval_policy`；切会话恢复。

### 2.5 /goal + GoalBar（D2）

```text
goal.get {workspace_id, conversation_id} -> {goal: GoalView|null}
goal.set {workspace_id, conversation_id, objective, max_rounds?} -> {goal}
goal.update {workspace_id, conversation_id, action: "pause"|"resume"|"cancel"} -> {goal}
```

- host 校验 workspace/conversation 归属，构建 `TurnContext` 调 `tool_goal::driver::{goal,set_goal,update_goal,cancel_goal}`；`pause` → update status `blocked` reason `paused by user`；`resume` → `active`；`cancel` → cancel。返回 JSON 解析为 Rust DTO（C10）。
- `GoalBar` 在 composer 上方：objective 摘要/status/rounds/max_rounds、暂停/恢复、清除（内联确认）；**`/goal <objective>` 设置也先内联确认**（显示 objective 摘要），取消不发 IPC；无 goal 不渲染；achieved/cancelled 只读；流式中不阻塞输入。
- 刷新：切会话/打开会话/命令后刷新；不新增事件通道；turn 结束由 app 主动 refresh（与 activity 轮询分离）。与 tool-goal 写同一文件，状态一致。

### 2.6 /export（D3 Markdown）

- IPC `session.export {workspace_id, conversation_id, format:"markdown"}` → `{path, bytes, mime, truncated:false}`。
- host：校验归属；读会话/消息/usage（store 既有 API）；生成 Markdown（标题/元信息/消息按 role/时间/usage/tool 行摘要；不包含附件二进制）；写 `<workspace>/.zeokeel/exports/<sanitized-title>-<yyyyMMdd-HHmm>.md`（原子写，文件名 sanitize/去控制字符）；>8 MiB 走 `export/too-large`；写失败 `export/failed`。
- 前端：**serve 用 Blob 下载**；**Tauri 无通用文件打开/opener** → 成功后展示并复制 workspace 相对路径（`system.open_url` 仅 http(s)，不用于本地文件、不承诺自动打开/打开所在位置）；不新增服务端。
- 空会话生成说明性 md；流式中导出已持久化快照并提示“导出不含进行中的流式内容”。

### 2.7 /feedback（D4）

- IPC `feedback.submit {workspace_id, conversation_id, category:"bug"|"idea"|"other", description?}` → `{stored:"local", path}`。
- host 校验分类/描述 ≤4 KiB、workspace 归属；写 `<workspace>/.zeokeel/feedback/<conversation>-<timestamp>.json`（含版本/edition/platform/时间，不含密钥/附件内容）；不写日志；不网络。
- UI：FeedbackDialog（ui-kit Modal）分类单选/描述/字数、明确「本地保存」；企业版提示“服务端反馈端点未实现（UNVERIFIED）”，不做“已发送”假状态；关闭/失败保留表单。

### 2.8 /file

- 复用 composer-surface 的附件选择/上传（`workspace.attachment_save`）与 chip；`/file` 仅触发选择器，不发送/不生成工具行；附件限制（8 类/1 MiB/5 个）与现有一致。

## 3. 备选与取舍

- D1 B（SQLite 表）：重启/切换更可靠，但需 v7 migration 且与 side v6 串行；Batch1 用 workspace 文件足够，后续可迁移。
- D3 ZIP：DSH 1:1，但下载路由/大文件/zip 校验成本高；Markdown 能覆盖“导出会话”用户价值，ZIP 留 Batch2。
- `/goal` 走 agent 工具：会生成工具行/可能触发审批/需模型轮次，不适合 UI 直接命令。
- Batch2 占位隐藏：会违背用户“菜单中显示未实现/禁用”的要求，不采用。

## 4. 安全与隐私

- 不新增 sandbox；`/permission` 文案不得暗示可限制员工（C6）。
- override 文件不含 key/URL；只存模型名/effort/policy；路径经 `safe_join_under` 类校验（复用 `zk-kit-tools` 单一 helper；host 经 generated facade）。
- export/feedback 只写当前 workspace；不跨 workspace；不写日志；描述不进入遥测；文件名 sanitize；大小上限。
- 新 IPC 全部 workspace/conversation 归属校验；错误码封闭；C10 schema；无 C2 运行期加载。

## 5. 生成物与门禁

- 契约变更：`cargo test --workspace export_schema` 后 `git diff` 作基线；`bash scripts/schema-check.sh`。
- `zk compose --edition personal --profile default --channel official --check`；`zk capability --check`（无新增）。
- `pnpm -r typecheck/lint/test:unit/build`；`bash scripts/i18n-check.sh`；`pnpm e2e`（m1 追加）；交互门禁工具落地后 `pnpm e2e:ui --grep "command"` + `scripts/ui-interaction-gate.py`（当前依赖 `ui-interaction-verification`，UNVERIFIED）。

## 6. 测试计划

| 层 | 用例 | 命令/证据 |
| --- | --- | --- |
| T0 组合 | schema/TS 生成、compose 不变 | `cargo test --workspace export_schema && git diff --exit-code`；`zk compose --check` |
| T1 host | override 读写/归属/损坏降级；**两会话并发 turn：全局 config-mutex 排队、A/B 各自 model/permission 不串、turn 后全局恢复、失败/取消回滚、settings revision 变化恢复最新、turn 运行中 settings.set 排队不污染当前 override、排队 turn 取消后跳过**；personal 手动模型/enterprise 列表校验；goal set/get/pause/resume/cancel；export 内容/上限/失败/文件名；feedback 分类/上限/失败；Modal 迁移后 ui-sessions 旧确认仍工作 | `cargo test -p zk-desktop`；`pnpm --dir packages/ui/ui-sessions/web test:unit` |
| T1 UI | 分组/过滤/键位/附件边界/未知命令/占位不执行；model（当前/手动/recent≤8/effort；enterprise 列表）；permission popup；GoalBar；feedback 表单；六态；ui-kit Modal 唯一 | `pnpm --dir packages/ui/ui-chat/web test:unit`；`pnpm --dir apps/desktop/web test:unit` |
| T3 e2e | `/file` 上传；`/model` 切换/切会话/重启；`/permission`；`/goal`+GoalBar；`/export` 文件断言；`/feedback` 文件断言；未知/占位；axe | `pnpm e2e`（m1-serve 追加）；交互证据 `target/evidence/2026-11-01-client-command-surface/interaction/<ID>.json` |
| T4 | i18n/type/lint/门禁 | 见 §5 |

## 7. 文档更新

- [ ] `docs/19` §2/§11：`/` DSH 命令面、键位、两 surface 分流、GoalBar 位置、命令错误/空态。
- [ ] `docs/26` §5：新 IPC 命令/错误码/override 文件/export/feedback 路径与限制；§2 数据文件约定。
- [ ] `docs/16` 由统一回写任务处理（本 spec 预算不含；ui-kit Modal 状态在 docs/19/26 引用）。
- [ ] `KNOWN-LIMITATIONS`：导出 Markdown（非 DSH ZIP）、反馈仅本地、会话 override 文件（非 DB）、**turn 全局串行/排队（吞吐下降，正确性优先）**、enterprise gateway 模型列表由 host 缓存校验、personal 无 provider 模型发现（手动输入）。

## 8. 风险

| 风险 | 概率 | 影响 | 缓解 | 验证 |
| --- | --- | --- | --- | --- |
| 全局 agent 配置串会话 | 中 | 高 | 全局 config-mutex：turn 前应用/turn 后恢复，其它 conversation turn 排队；失败/取消回滚；settings revision 校验 | host 两会话并发/e2e 切会话 |
| 菜单/⌘K 回归 | 中 | 中 | 单一 catalog + surface 过滤；旧 ⌘K 断言 | 单测 |
| 未知命令被发送 | 中 | 高 | 解析器拒绝 + 测试；草稿保留 | UI/e2e |
| export 大文件/路径逃逸 | 低 | 高 | 8 MiB + `safe_join_under` + workspace 归属 | host 单测 |
| feedback 假成功 | 中 | 中 | 只本地 + 明确文案；服务端 UNVERIFIED | e2e/文案 |
| DSH 权限语义误宣传 | 中 | 高 | UI/文档明确“仅审批、无沙箱、不限制员工” | 评审 |
