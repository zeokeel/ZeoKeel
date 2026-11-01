# 客户端 vs DSH 能力/配置面对比审计（2026-11-01）

> 目的：回答「ZeoKeel 客户端相对 DSH 差在哪、为什么大部分能力在 UI 里看不到/配不了」。
> 性质：**事实盘点 + 缺口矩阵**（C19 前置材料）；产品取舍与施工范围由 spec 决定。
> 审计对象：ZeoKeel `HEAD de17d0e`（1.0.0-rc.1，personal 组合）vs 本机 DSH **0.2.1-rc.2**（当前 `web` profile，即本 GUI）。
> 证据文件（审计原始产物，`target/` 已 gitignore）：
> - `target/audits/zk-client-inventory.md`（607 行，299 处 `path:line`）
> - `target/audits/dsh-reference-inventory.md`（1116 行，43 条已抽检 + 18 条沿用 `research/dsh-ui-modules`）
> C15：DSH 仅作事实参考，不作为依赖/构建输入/索引仓/数据源；本文不复用其代码/词条/资产。

## 0. 一页结论

1. **后端能力远大于 UI 暴露**：personal 组合启用 **41 个运行时工具**、6 个 `cap-*`、12 类 AgentEvent，
   但 UI 只注册 **7 个真实 slot**（+5 个企业 stub），27 个 UI 面；41 个工具里只有 2 个有专属渲染，
   **12 个编排/后台工具连可管理面都没有**（goal/ralph/workflow/subagent×4/agent_team/todo/present）。
2. **「不能配置」的核心是设置面结构债**：全部 40 个 `AppSettings` 叶子字段挤在**一个 618 行单页 dialog** 里，
   无分页、无扩展点（唯一注入是 provider 表单）、`settings.set` 全量提交无 revision 栅栏；
   且真正缺入口的可配置项是 **MCP server、审批策略、子代理上限、终端/Shell、jobs 上限、schedule 任务管理**。
3. **三处「配了不生效」必须诚实处理**：`auto_approve_read_only` 全仓无运行时消费者；`embedding`/`media`
   四类凭据可编辑但 `cap-*` 无任何 tool/agent 调用（能力悬空）；`usage` 事件被 UI 丢弃（靠 turn 结束汇总兜底）。
4. **P0 正确性/安全问题不是缺面，而是「看起来有」的降级面**：审批卡缺 `docs/19` §4.1 的 9 项安全字段；
   刷新/重进会话工具行丢失、`role="tool"` 消息被渲染成助手原始 JSON；i18n 门禁当前红（`main.tsx:47,49`）。
5. **DSH 的可比对象很小**：DSH user-facing 设置 ≈ 30 项（5 分区 + 12 通用行 + 4 插件配置页），
   其余 511 个 schema 字段是**部署级 cordis config**，对位 ZeoKeel 的 `zk.toml`/构建期组合（C2），不该塞进设置 UI。
   DSH 的插件市场/运行期装包与 C2 冲突，**不可移植**。

## 1. 方法与证据口径

| 项 | 内容 |
| --- | --- |
| ZeoKeel 侧 | 只读源码 + 生成物 + 契约 + e2e/测试；结论以代码为准，文档冲突单列 §6 |
| DSH 侧 | 只读安装包 `E=/usr/local/lib/node_modules/@deepseek-ai/dsh/node_modules/@deepseek-ai/` 的 README.zh / locale / `lib/client.js`；`dsh --profile web --dump-config`（1317 行）与 `--dump-config-schema`（915,818 B） |
| DSH 口径 | 「已抽检 43 / 沿用调研 18 / 未验证 8」；web profile 当前 bundle：`dsh-base`、`dsh-web-app` + 7 个 experimental bundle |
| 未覆盖 | 未启动 Tauri/`serve` 实跑；未构建 enterprise 组合（企业 UI 渲染仅按 `editions`+registry stub 推断）；DSH 侧未在 GUI 手工点测（按包内材料抽检） |

## 2. 数量对比

| 维度 | DSH web 0.2.1-rc.2 | ZeoKeel personal 1.0.0-rc.1 |
| --- | --- | --- |
| UI 包 | 57（52 静态组合 / 5 未组合 / 1 个 web 运行期禁用） | 12（personal 组成 8：7 slot + kit） |
| 已注册 slot | 95 声明（69 个有启用占用者） | 7 真实 + 5 `null` stub |
| 设置面 | 5 导航分区 + 12 「通用」行 + 侧栏插件页 4 配置页 + 1 只读插件清单 | 1 个单页 dialog：9 fieldset + 3 附加块 |
| 用户可配置项 | ≈30 项（另 511 个部署级 schema 字段不属用户设置） | 40 叶子字段（personal 可编辑 37，enterprise 23） |
| 主界面 | 三栏 AppFrame + 3 个 keyed 主面板 + per-session 右栏停靠面（8 类 tab） | shell + 侧栏 + 单对话 + 右栏检查器（3 固定 tab） |
| 聊天渲染器 | 19 种 chat node + 54 种 tool view + turnTail 交付卡 | assistant-ui thread；工具行 1 种通用卡 + 审批/提问/usage 3 个部件 |
| 命令 | 6 Host + 2 客户端 + 技能目录 + `@` 引用；**无全局命令面板** | ⌘K 面板 4 条（personal）/7 条（enterprise） |
| 快捷键 | 18 条可配置 + 固定键（Enter/Shift+Enter/Ctrl+Enter/Esc Esc…） | 2 条全局（⌘B、⌘K）+ 组件级 Esc/Enter/↑↓ |
| 主题/字体 | light/dark/system + 7 个字体/字号字段 | auto/light/dark（无字体/字号设置） |
| 后台/编排 UI | jobs、subagent catalog、Agent Teams、workflow run、schedule 页、deliverables、attachments | 无（仅聊天内生工具行） |
| 文件/终端 | 文件树（懒加载）+ 文档预览（文本分页/PDF/HTML/图片/Office）+ 真实 xterm + 浏览器 tab（Desktop） | 文件平铺只读、无预览；终端仅占位文案 |
| MCP/连接器 | **无用户面**（`dsh-mcp-client` 未组合） | 后端有 MCP client（personal 已启用），但只能 `ZEOKEEL_MCP_SERVERS` env；企业走 catalog |

## 3. 能力面对比矩阵

状态：`✅` 完整可用；`🟡` 有面但降级/项不全；`⛔` 无面；`➖` 不适用/不可移植。

| 能力面 | DSH | ZeoKeel 现状 | 缺口定性 |
| --- | --- | --- | --- |
| 设置导航/分页 | 5 分区 + 12 行 + 4 插件页 + 1 清单（`ui-settings-general` 等） | 单页滚动 dialog（`packages/ui/ui-settings/web/src/index.tsx:165-606`） | ⛔ 结构债（用户诉求核心） |
| 模型/Provider | 模型页：provider 行、API key 写-only、模型列表、协议、推理等级、发现模型 | provider 表单 + completion 参数；无模型列表/发现/多 provider 行 | 🟡 |
| Agent 循环 | 插件页「Agent 循环」：并行工具调用上限 | 设置页 `parallel_tool_calls` 等（已可配） | 🟡 可配项少 |
| 子代理 | 插件页「子智能体」：最大深度/并行数/模型选择白名单 | 无设置项；工具 `subagent*` 存在（`docs/28` 后置） | ⛔ |
| 终端/Shell | 插件页「终端」：命令超时/单流输出上限 | 无设置项；右栏终端是占位文案（`ui-inspector:77`） | ⛔ |
| 网页搜索 | 插件页「网页搜索」：开关/key/baseURL/maxUses/count | 已可配 baseURL/timeout（`ui-settings:405-432`），无 key/maxUses/count | 🟡 |
| 权限/审批 | 权限预设 3 档 + 审批策略（`approval.policy`）；审批面板仅 allow-once/reject | `auto_approve_read_only` 字段无控件且**无消费者**；审批卡只有 tool/args/reason + 批准/拒绝 | ⛔ 配置无效 + 安全字段缺失 |
| MCP/连接器 | 无用户面（客户端未组合） | personal：仅 env；enterprise：catalog 下发 + 启停/授权，但 `connectors.configure/reload/clear` 未进契约、Web 零调用 | ⛔（ZeoKeel 能力领先、UI 落后） |
| 调度/自动化 | 「自动化任务」页 + 右栏任务 tab：列表/搜索/改名/改时间/运行记录/删除 | 仅全局开关 + 扫描间隔（`ui-settings:377-394`）；任务文件 `.zeokeel/schedules.json` 无 UI | ⛔ |
| Jobs/后台任务 | 会话头任务列表 + 输出面板 + 停止 | 3 个 `job_*` 工具、无面板 | ⛔ |
| 子代理/团队 | header 目录 + 续聊 + `@` 引用；Agent Teams roster/任务板 | 6 个 subagent 工具 + agent_team，无任何面 | ⛔ |
| Workflow/Ralph/Goal/Todo | 原生节点/turnTail/GoalBar/Todo dock | tool 存在，无面（grep 0 命中） | ⛔ |
| Plan mode | `/plan` + 右栏 review + turnTail 计划卡 | 无 | ⛔ |
| 交付物/deliverables | turnTail 改动文件卡/交付卡 + changes-review tab | 无；`present` 工具无渲染 | ⛔ |
| 附件 | composer 附件栏/拖放/图廊/灯箱 | 无（`docs/28` 后置） | ⛔ |
| `/` `@` 触发 | `/` 命令菜单 + `@` 文件/会话引用 | 无（composer 仅输入 + 发送/停止） | ⛔ |
| 模型选择 | composer 模型控件 + `/model` | 仅企业设置里 gateway model 下拉 | 🟡 |
| 命令面板/快捷键 | 无 ⌘K；18 条可配置快捷键 + 速查/录制 | ⌘K 4 条；2 条全局快捷键；无速查/自定义 | 🟡（ZeoKeel ⌘K 是自有增强） |
| 右栏 | per-session、多 pane/float/fullscreen、8 类 tab（files/text/terminal/browser/changes-review/plan/schedule/subagent） | 固定 288px 单栏：files（平铺）、changes、terminal（占位） | 🟡 |
| 文件浏览/预览 | 懒加载文件树 + 文本分页/PDF/HTML/图片/Office 预览 | 平铺路径列表，无树/无预览 | ⛔ |
| 主题/字体 | 3 主题 + 7 字体/字号设置 | 3 主题，无字体/字号 | 🟡 |
| 诊断/门禁/更新 | 连接状态、Host/页面版本不一致弹窗、Desktop 更新行、账号门禁 | 仅 `ui.gate.update`；无诊断页；个人版更新只有「检查」文案 | ⛔ |
| 账号/余额 | DeepSeek 账号页（登录态/余额/充值/退出） | 企业版自家登录 + 设置里登出；personal 无登录 | ➖（后端不同） |
| 插件管理/市场 | 运行期 pnpm 装包/启停/卸载 + 市场 | 构建期组合（C2）；企业商店分发技能/连接器/应用 | ➖ 不可移植 |
| 语音输入/CoT 翻译/浏览器 | web 基线未启用/Desktop-only | 无 | ➖（DSH 基线也没有） |
| Skills | 本地 skills 目录 + `/<skill>` | `tool.skill` 可被模型调用；个人版无浏览面，企业版有商店技能面板 | 🟡 |
| Agent 预设 | 4 内置预设 + 会话级默认 | 构建期 `edition/profile/channel`（C2） | ➖ 不同层，仅「新会话默认」交互可参考 |
| Workspace/会话 | 分组/树/ungrouped/archive-only | 平铺 workspace、会话无 rename/fork/archive/pin；无全文检索 UI | 🟡（C18 约束下不照搬） |

## 4. 缺口清单（按优先级）

### P0（安全/正确性，建议先修）

| # | 缺口 | 证据 |
| --- | --- | --- |
| P0-1 | 审批卡缺 `docs/19` §4.1 的 9 项安全字段（动作摘要/完整命令/工作目录/读写范围/diff/风险级别/剩余时间/始终允许/记住范围/默认焦点拒绝）；`Cancel`、「始终允许本会话」不可达 | `packages/ui/ui-chat/web/src/ApprovalPart.tsx:29-68`；`packages/contracts/agent/src/lib.rs:163-172`；`apps/desktop/web/src/state/controller.ts:670-679` |
| P0-2 | 刷新/重进会话工具行丢失；`role="tool"` 消息被当作 assistant 原文渲染（出现 `{"name":…}` JSON 气泡） | `packages/contracts/session/src/lib.rs:55-60`；`apps/desktop/src/events.rs:65-76`；`apps/desktop/web/src/state/reducer.ts:140-152` |
| P0-3 | i18n 门禁红：`main.tsx` 硬编码中文（AGENTS.md §6 声称门禁绿） | `bash scripts/i18n-check.sh` → exit 1；`apps/desktop/web/src/main.tsx:47,49` |
| P0-4 | `auto_approve_read_only`「能存能读、有 i18n 文案、无控件、全仓无消费者」 | `packages/contracts/config/src/lib.rs:587`；`packages/ui/ui-settings/web/src/settings-validation.ts:199,323`；`rg auto_approve_read_only` 无运行时消费 |

### P1（用户明确诉求：配置面 + 能力可见面）

| # | 缺口 | 证据 |
| --- | --- | --- |
| P1-1 | 设置 IA：40 字段单页、无分页/扩展点、`settings.set` 全量文档无 revision 栅栏 | `ui-settings/web/src/index.tsx:165-606`；`packages/contracts/ipc/src/lib.rs:240-248` |
| P1-2 | MCP server 配置无 UI（personal 仅 env；enterprise catalog 下发不可见/不可改），`connectors.*` 未进契约、Web 零调用 → 连接器启停后 MCP catalog 不重载 | `packages/client/connector-mcp/README.md`；`apps/desktop/src/ipc.rs:630-650`；`KNOWN-LIMITATIONS.md:99-100` 与代码冲突 |
| P1-3 | 子代理/团队/workflow/ralph/goal/todo/present/jobs 共 12 个工具无管理面（无状态/取消/结果面板） | grep `goal|ralph|workflow|subagent|agent_team|job|todo` 于 `packages/ui/*` + `apps/desktop/web/src` 命中 0 |
| P1-4 | schedule 任务无列表/新建/编辑/暂停/删除 | `ui-settings/web/src/index.tsx:377-394`；`packages/client/tool-schedule` |
| P1-5 | composer 缺 `/` 菜单、`@` 引用、附件、模型下拉、上下文占用、queue/steer | `packages/ui/ui-chat/web/src/AssistantComposer.tsx:1-51` |
| P1-6 | 右栏真实终端缺失（占位）、文件无树/无预览、无宽度/多 pane | `packages/ui/ui-inspector/web/src/index.tsx:31,65-77` |
| P1-7 | `cap-image/audio/transcription/rerank/vector` 配置可编辑但 agent 无调用入口（能力悬空，易被误当已交付） | `packages/client/cap-*/src/lib.rs` 仅 `configure*`；`rg zk_cap_` 于 `tool-*`/`agent-rig` 无命中 |
| P1-8 | 个人版更新链路只回文案：无下载/校验/安装/回滚（`tauri.conf.json` 无 updater、`bundle.active=false`） | `apps/desktop/web/src/App.tsx:435-445`；`apps/desktop/tauri.conf.json` |
| P1-9 | `usage` 事件被 UI 丢弃（仅 turn 结束汇总可用） | `packages/contracts/agent/src/lib.rs:140`；`apps/desktop/web/src/protocol.ts:669-670` |
| P1-10 | 会话级操作缺 rename/fork/copy ID/pin/archive；无会话全文检索 UI | `packages/ui/ui-sessions/web/src/index.tsx`（workspace 操作齐全、会话无菜单） |

### P2（体验/工程补齐）

| # | 缺口 | 证据 |
| --- | --- | --- |
| P2-1 | 命令面板仅 4/7 条；无快捷键速查/自定义 | `apps/desktop/web/src/App.tsx:571-583` |
| P2-2 | 主题无字体/字号；无 OEM accent 之外的细粒度外观 | `ui-settings:239-260` |
| P2-3 | 门禁只覆盖 update；缺 account/maintenance/channel | `apps/desktop/web/src/UpgradeGate.tsx`；`docs/19:26-28` |
| P2-4 | 无诊断页/日志导出/包清单 UI | `ui-shell:156-160`；`docs/19:175-179` |
| P2-5 | 工具行缺 pending/success/failed/cancelled、耗时、Shift 全展开 | `packages/ui/ui-chat/web/src/ToolCallPart.tsx:5,20-26` |
| P2-6 | 托盘/通知/深链未实现 | `apps/desktop/src/main.rs`、`tauri.conf.json` |
| P2-7 | 设置保存失败/多窗口并发覆盖风险（无 revision） | `ipc/src/lib.rs:240-248` |

## 5. 不可移植 / 需按 ZeoKeel 约束重设计（C1–C19）

| DSH 事实 | ZeoKeel 约束 | 结论 |
| --- | --- | --- |
| 运行期插件安装/启停/卸载 + 插件市场 | C2 构建期组合、运行期不加载 | 不移植；只对位「只读内置清单」+ 企业商店分发 |
| 多协议自定义 provider（Chat Completions/Anthropic/Responses/pi-ai） | C7 企业只走网关且只支持 Responses | 仅 personal 本地 provider 可扩展，企业版保持窄 |
| 权限预设/审批策略是 DSH 个人安全模型 | C6 企业弱管控、不做企业审批 | 审批/权限只做客户端本地策略（personal + enterprise 本地工具审批） |
| DeepSeek 账号/余额/充值 | C8 无部门/用户组、角色三档；企业版自家登录+钱包 | 不照搬账号页；企业版登录/钱包另有规范 |
| Workspace 分组/ungrouped/archive-only | C18 无工作区不建会话、`workspace_id` 不可变、无 ungrouped | 不照搬；只参考搜索/分组的信息密度 |
| Agent 预设（运行期会话级选包） | C1/C2 构建期 edition/profile | 不同层；只参考「新会话默认」交互 |

## 6. 文档 ↔ 代码冲突（施工前需先修文档，C12/C19）

| # | 文档说法 | 代码事实 | 证据 |
| --- | --- | --- | --- |
| 1 | `KNOWN-LIMITATIONS.md:99-100`：connectors 更新事件「web 侧接线」 | Web 零调用；三条命令也不在 IPC 契约 | `rg connectors.` 于 `apps/desktop/web/src` 无命中；`ipc.rs:630-650` |
| 2 | `docs/16:16` 「M1 包结构（active 6）」 | 实际表内 8 包 + 薄壳，且未列 enterprise 4 个 UI 包 | `docs/16:16-28`；`packages/ui/ui-login` 等 |
| 3 | `docs/16:109` 把 login/skills/connectors/apps 列为 post-M1 未实现 | 包已实现并在 enterprise 组合启用（personal 生成物为 stub） | `editions/enterprise.toml:55-68`；`generated/registry.ts:23-27` |
| 4 | `docs/28:97` shell/inspector/commands 未接入 | 已接入 | `apps/desktop/web/src/App.tsx:614-676` |
| 5 | `docs/19:61` 仅 3 个 slot | 实际 7 个 | `generated/registry.ts:17-26` |
| 6 | `docs/28:72` 设置含「自动批准只读」 | 控件不存在 | `ui-settings` 全文无该 input |
| 7 | AGENTS.md §6 门禁绿（含 i18n） | `i18n-check` exit 1 | 实测 `main.tsx:47,49` |
| 8 | `connector-mcp/README.md` HTTPS/TLS UNVERIFIED | `KNOWN-LIMITATIONS.md:93-95` 称已实现 rustls | 两文档互相冲突（非 UI 项） |
| 9 | `docs/19:34` 审批是占位卡 | 审批卡已可决策，但缺 §4.1 字段 | `ApprovalPart.tsx` |

## 7. 建议的施工拆分（供拍板；每项一个 spec）

| 阶段 | spec 建议 | 范围 | 估算文件 |
| --- | --- | --- | --- |
| Phase 1 | `client-settings-ia` | 设置信息架构：左侧分区导航（通用/模型/Agent/工具与审批/连接器/自动化/外观/关于）+ pane 扩展点（`ui.settings.pane` 贡献）+ 分节保存 + revision 栅栏；现有 40 字段搬迁，不新增配置语义 | ~15 |
| Phase 1 | `client-config-approval-mcp` | 修复 `auto_approve_read_only` 消费链（审批策略）；MCP server 本地配置（契约 `AppSettings.mcp` + UI CRUD + 测试连接）；`connectors.*` 进契约并接线 | ~25 |
| Phase 1.5 | `client-approval-tool-rendering` | P0：审批卡 §4.1 全字段、工具行持久化投影 + `role=tool` 渲染、`usage` 事件、i18n 门禁 | ~15 |
| Phase 2 | `client-capability-surfaces` | composer `/`+`@`+附件+模型选择+上下文；右栏真实终端/文件树/预览；jobs/subagent/goal/todo/schedule/present 面板；会话 rename/fork/archive + 全文检索入口 | ~40 |
| Phase 3 | `client-settings-agent-tools` | 子代理/团队/终端/Shell/jobs 配置段（契约 + 设置 pane + 运行时接线）；个人版更新链路；诊断页/门禁/快捷键/主题字体 | ~30 |

> 注：Phase 1 的两个 spec 是「用户诉求（UI 上能配置）」的直接回应；Phase 1.5 是安全/正确性修复；Phase 2 才补「能力可见面」。
> 每阶段结束必须回写 `docs/16`/`docs/19`/`docs/28`（当前口径已过期，见 §6）并跑 `docs-check`/`spec-check`/`i18n-check`。

## 8. 复现命令（只读）

```bash
# ZeoKeel：设置字段与控件、审批卡、工具行、i18n
rg -n 'id="settings-' packages/ui/ui-settings/web/src/index.tsx | wc -l
sed -n 29,68p packages/ui/ui-chat/web/src/ApprovalPart.tsx
sed -n 130,160p apps/desktop/web/src/state/reducer.ts
bash scripts/i18n-check.sh; echo "exit=$?"

# ZeoKeel：组合与 slot
sed -n 1,80p editions/personal.toml
cat apps/desktop/web/src/generated/registry.ts

# DSH：组合、设置、schema
dsh --profile web --dump-config > /tmp/dsh-audit/dump-config.yml
dsh --profile web --dump-config-schema > /tmp/dsh-audit/schema.json
for d in /usr/local/lib/node_modules/@deepseek-ai/dsh/node_modules/@deepseek-ai/dsh-client-ui-settings*; do
  head -8 "$d/README.zh.md"
done
```
