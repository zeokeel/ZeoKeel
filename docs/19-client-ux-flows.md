# 19 · 客户端 UX 流程与页面契约（补充规范）

> 状态：Plan v3.2 补充规范。本文补齐 `01/05/11/16` 中"有原则、无旅程"的缺口：
> **每个页面/状态归哪个包、哪个槽位、显示什么、失败怎么恢复、如何验收**。
> 与 `16-ui-packages.md` 的 UI 包模型配合；冲突时本文对"客户端交互"优先，`05` 对运行时优先。

## 1. 应用启动状态机

```text
Boot
 ├─ 读取构建期常量（EDITION/CHANNEL/SERVER_URL_DEFAULT）
 ├─ personal：本地数据检查 → Onboarding（首次）或 Chat
 ├─ enterprise：
 │   ├─ 渠道 server_url 非空 → Login（地址锁定）
 │   ├─ 渠道 server_url 为空 → 首配页（输入地址并保存）
 │   └─ 有本地会话 → 校验版本/设备 → Chat
 ├─ 包注册/激活（按 required 分流；降级包显示 fallback，见 05 §3）
 └─ 失败 → ErrorScreen/Diagnostics（可重试/退出/导出）
```

强制门禁（Gate）优先于一切业务页面：

| 条件 | 页面 | 可用动作 |
| --- | --- | --- |
| `CLIENT_TOO_OLD` / 426 | `ui.gate.update` | 打开下载页、查看版本说明（当前/最低版本）；**不重试业务请求**；不进入聊天 |
| 账号禁用/令牌撤销 | `ui.gate.account` | 重新登录、联系管理员、导出本地数据 |
| 服务端维护中 | `ui.gate.maintenance` | 重试、查看公告；维护页只拦截服务端依赖功能，本地 agent 可用性待定（见 [15 §C](15-open-questions.md#c-待定清单)） |
| 渠道/版本不匹配 | `ui.gate.channel` | 仅显示错误与支持信息，不提供"手动下载"（防串渠道） |
| 包启动失败 | `ui.diagnostics` | 查看包 id/错误链/日志导出/重启 |

## 2. 页面 / 槽位 / 状态（冻结）；归属包 id 示意（T2 冻结）

> **工作区前置**：所有对话在 Workspace 内进行（`26`，M1 代码待实现）。首用/切换/目录不可用规则见 `26` §6；本文不复述。
> **M1 范围**：个人版 onboarding（`19` 引导页）与审批完整流程为 post-M1；M1 设置弹窗覆盖 provider 配置，审批是占位卡。
> 视觉/六态/证据见 `27`；UI 包见 `16`。

| 页面 | slotId | 归属包（示意） | 必测状态 |
| --- | --- | --- | --- |
| 启动/加载 | `app.boot`（页面 id，非 slotId） | T2 冻结（shell 提供） | loading、包失败、超时 |
| 个人版引导 | `ui.onboarding` | `client-onboarding-personal` | 首次、已配置、key 无效、网络不通 |
| 登录 | `ui.login` | `client-auth` | 空、加载、错误、锁定、强制改密、OEM 地址锁定 |
| 首配服务端地址（官方企业版） | `ui.slot.login`（首配变体；旧称 `ui.first-run`） | `client-auth` | 地址非法、不可达、证书错误、保存成功 |
| 主界面（M1: ui.shell + 三内容槽） | — | `@zeokeel/ui-shell` + app 薄壳 | 正常、离线、连接中、断线重连中 |
| 工作区切换/首用 | `ui.sessions` | `ui-sessions` | 无工作区、加载中、目录不可用、切换/取消流式 |
| 对话 | `ui.chat` | `ui-chat` | 六态 + 长会话 + 取消/重试 |
| 输入 | `ui.composer` | `ui-composer` | 空、 composing（IME）、附件中、超长、发送失败 |
| 工具调用 | `ui.toolcall` | `ui-tools` | pending/running/success/failed/cancelled |
| 审批 | `ui.approval` | `ui-approval` | 等待、超时、批准/拒绝、记住选择 |
| 设置（分页） | `ui.settings` + list slot `ui.settings.pane` | `ui-settings`（外壳）+ 各 pane 贡献包 | 保存中、失败、需重登、**revision 冲突**、空 pane 列表、pane 渲染异常局部降级 |
| 更新 | `ui.update` | `client-update` | 检查中/有新版/下载/校验/安装/重启/失败/回滚 |
| 诊断 | `ui.diagnostics` | `client-diagnostics` | 包清单、错误链、日志导出、重启 |
| 技能/连接器 | `ui.skills`/`ui.connectors` | `client-skill`/`client-connector` | 空/加载/错误/无权限/离线/已禁用 |
| 应用（仅企业版） | `ui.apps` | `client-wasm-host` | 空/加载/错误/无权限/离线/已禁用 |
| 更新门禁 | `ui.gate.update` | `client-update` | 426/强制更新、下载、回滚 |
| 账号门禁 | `ui.gate.account` | `client-auth` | 禁用/撤销、重登、导出 |
| 维护门禁 | `ui.gate.maintenance` | `ui-shell` | 维护、重试、离线、公告 |
| 渠道门禁 | `ui.gate.channel` | `ui-shell` | 渠道/版本不匹配、无手动下载 |

规则：

- **M1 仅实现 `ui.sessions`/`ui.chat`/`ui.settings` 三个 slot**（`16` §2.2）；本表其余 slot 为 post-M1/T21 目标，不得作为 M1 验收。`app.boot` 是页面 id 例外。
- 归属包 id 为示意（T2 冻结）；本表冻结页面、slotId 与必测状态。
- 页面只能通过 `slotId` 注册；shell 不 import 具体页面组件。
- `client-auth` 在企业版必选、个人版排除；`client-onboarding-personal` 反之。
- 每个 `ui.gate.*` 必须有 fallback 文案与"下一步"动作；不允许白屏。
- 登录页品牌文案来自 `channel.json` 的 `copy.login_*`，缺键即构建失败（禁止回落官方）。

### 2.0 右栏与会话操作（Phase 2b，`2026-11-01-client-side-panels`）

- **右栏**（`ui.slot.inspector`，⌘B）：tabs 顺序 files 10 / changes 20 / terminal 30（activity 40 预留）；tabs 支持 ←/→/Home/End；files 为**懒加载树**（展开目录只取直属子项，隐藏 `.git`/`.zeokeel`，符号链接不跟随/不展示，目录行 loading/error+行内重试，刷新保持已展开路径）；预览文本 ≤256 KiB（截断标「已截断」）、图片 png/jpeg/webp/gif ≤8 MiB，PDF/Office/HTML/二进制诚实降级；terminal 是**用户显式**命令（open/send/close/list，不生成工具行/审批，输出 ≤64 KiB、超时 ≤10 s、无 shell 显示「终端不可用」）；tab/宽度（240–720 且 ≤视口 50%，键盘可调）/折叠按**会话**持久化，窄窗 <768 自动收起保留展开状态。六态 + axe/键盘基线见交互契约。
- **会话侧栏**：会话行 `⋯` 菜单（rename 非空 ≤200 / fork 同 workspace / copy id 带成功提示 / pin 置顶 / archive 隐藏不删）；「显示已归档」筛选可恢复；流式中 archive/fork 禁用；搜索框默认标题模式，切「内容」模式调 `session.search`（当前 workspace、防抖 ≥250 ms、取消旧请求、snippet 结果按会话展示、点击打开会话、归档命中带标记）。C18：所有会话操作/检索不跨 workspace。

### 2.1 composer 输入面（Phase 2a，`2026-11-01-client-composer-surface`）

> 状态：已实现（交互契约 `specs/changes/2026-11-01-client-composer-surface/interaction.md`；证据
> `target/evidence/2026-11-01-client-composer-surface/`）。composer **仍留在 `ui-chat`**，不新增 `ui.composer` slot。

输入行（`AssistantComposer`）在既有 Enter/发送/停止/字数/离线提示之上，新增：

- **`/` 命令菜单**：词首输入 `/`（非 IME composing）打开候选，候选 = **与 ⌘K 同一份 app 命令目录**（同一 `runCommand`；
  personal 6 条 = 新建会话/设置/侧栏/右栏/取消/重试，enterprise 再加技能/连接器/应用 3 条 = 9 条）；↑/↓/Enter/Esc、过滤不区分大小写；
  执行后清除 `/query`、焦点回输入框、正文保留；`disabled` 命令可见不可执行（`/cancel` 仅流式中、`/retry` 仅有上一条发送文本且非流式）。
- **`@` 文件引用**：词首输入 `@` 或点「引用文件」按钮打开 workspace 文件候选（来自既有 `workspace.files`，不新增搜索 IPC；
  隐藏 `.git`/`.zeokeel`）；过滤 path（不区分大小写）、目录标 `dir`、目录以 `@path/` 形式入 chip；选择后生成可删除的**引用 chip**
  并从输入框移除 `@query`；重复去重、按添加顺序；引用+附件合计 ≤5 条/消息。
- **附件**：拖放或「添加附件」文件选择；客户端快速校验（8 类 allowlist：png/jpg/jpeg/webp/gif/pdf/txt/md/json/csv/log；
  单文件 ≤1 MiB；≤5 个/消息；空文件拒绝）；图片缩略图用 `FileReader.readAsDataURL` 生成 **`data:` URL**（非 `blob:`，不改 CSP）；
  上传中/失败 chip 可移除或重试，失败不丢草稿、不发送半成品。附件由宿主保存到
  `<workspace>/.zeokeel/attachments/<conversation_id>/<uuid>-<sanitized-name>`（`workspace.attachment_save`），
  路径**必须**经 `zk_kit_tools::safe_join_under` 校验（逐组件 no-follow / canonicalize 父目录 / `starts_with(root)` / 拒绝绝对与 `..`；
  父目录 symlink 指向工作区外时拒绝写入）。附件仅作为 workspace 文件 + 引用块进入消息，不做多模态字节直传。
- **模型只读标签**：composer 底部显示当前模型（personal = `AppSettings.provider.model`，未配置显示「未配置模型」；
  enterprise = 已选 gateway model，未选显示「未选择网关模型」）；点击打开设置并**定位到模型相关 pane**
  （`openSettings(paneId)` 深链，personal `provider` / enterprise `enterprise`），本轮不做内联切换/模型发现。
- **用量指示**：只读显示本轮 input/output/cached/total 与会话累计（当前会话已持久化 assistant usage 求和），
  数据仅来自既有 `AgentUsage`/`usage` 事件；无 usage 显示「用量未报告」（不显示 0、不估算上下文窗口百分比）。
- **发送与引用块**：发送文本 = 用户正文 + 尾部规范化引用块（`--- referenced files ---` + 每行 `@<workspace 相对路径>`；
  path 以 `@` 开头写 `@@path`；拒绝控制字符/换行/绝对/`..`；块只允许在消息尾部）。无正文但有引用/附件时允许发送；
  引用块之外的正文按既有规则渲染；UI 把引用块渲染为 chip/列表（不展示原文），刷新/回放后仍识别。`chat.send` 契约不变。
- **键盘/IME/六态**：Enter 发送、Shift+Enter 换行、Esc 关闭菜单且不丢草稿；菜单打开时方向键只移高亮、焦点留在输入框；
  `compositionstart→end` 期间 Enter/`/`/`@` 不触发发送/菜单；`/`/`@` 仅词首触发；六态（空/加载/错误/离线/流式中/已取消）不白屏；
  axe 0 serious/critical。

## 3. 关键旅程（验收口径）

### 3.1 个人版首次使用

1. 安装 → 启动 → 引导页（欢迎 + provider 选择 + key 输入 + "测试连接"）。**M1：引导页为 post-M1，先用设置弹窗完成 provider 配置。**
2. 测试连接成功 → 进入 Chat；失败 → 显示错误类型（网络/401/模型不存在），key 不落日志。
3. 首次对话 → 流式回复 → 工具调用 →（审批 post-M1）→ 结果写入会话 → 重启后可恢复。
4. 更新提示 → 下载/校验/安装/重启；失败可回滚。

验收：无服务端依赖；断网时本地 agent/会话可用；key 在 M1 暂存本地 JSON（0600），M2 接系统密钥库；首 token < 2s（mock 基线）。

### 3.2 企业版 OEM（地址预置）

1. 门户下载 → 安装 → 启动 → 登录页**只有账号/密码**（不显示地址）。
2. 登录成功 → bootstrap → Chat；失败按错误码显示可读文案（不泄露内部信息）。
3. 强制更新 → Gate 页 → 从当前服务端下载 → 校验 → 安装 → 重启 → 登录态保持（或要求重登，按策略）。

### 3.3 企业版官方（地址为空）

1. 启动 → 首配页：输入服务端地址 → 校验 https/可达性/渠道 → 保存本地。
2. 登录页只读展示已保存地址；修改地址在**设置**中进行，保存后需重新登录。
3. 服务端地址不可用时提供"重试/去设置修改/导出诊断"，不自动扫描局域网。

### 3.4 设置弹窗（分页 + 单一保存 + revision 冲突）

> 状态：**已实现**（`specs/changes/2026-11-01-client-settings-ia`，2026-11-01；证据 `target/evidence/2026-11-01-client-settings-ia/`）。

入口：侧栏「设置」或 ⌘/Ctrl+K →「设置」。Dialog 保持 M1 形态（不做独立路由/窗口）。

- **布局**：左导航（每个 `ui.settings.pane` 贡献一项，按 `order` 升序）+ 右侧当前 pane；默认选中第一个可用 pane。
  窄窗（< 720px）导航退化为顶部横向 tab，内容区不横向滚动。企业地址锁定提示、退出登录、企业模型在「企业」pane（仅 enterprise 贡献）。
- **MCP 本地服务（client-mcp-config）**：personal-only pane「MCP 本地服务」（`ui.settings.pane` order 70，enterprise
  构建期不存在）：列表/新增/编辑/删除/测试连接（`mcp.test` 轮询）/保存；空态引导添加，测试中该行 pending、按钮禁用，
  连接失败行内错误；关闭设置丢弃草稿（单次底部保存）。配置非 secret，本地 server 不允许 OAuth（提示用企业 catalog）。
- **审批策略（config spec C-4）**：审批策略控件位于「工具与审批」pane（order 40，两 edition 共用）：
  `ask`（默认）/`auto_read_only`/`strict` 三选一；文案说明「任何策略都不会自动批准声明需要审批的工具；strict 会询问只读/交互工具」；
  保存后立即生效（runtime settings 发布），旧 `auto_approve_read_only` 不单独呈现（只做迁移）。
- **pane 来源**：全部内建 pane（通用/模型/本地 Provider/Agent/工具与审批/网络与检索/嵌入/媒体/企业/关于）都是
  `ui.settings.pane` 贡献；`editions` 在构建期过滤（personal 无企业 pane；enterprise 无 provider/embedding/media），pane 内无 edition 分支。
  无任何 pane 贡献时显示空态 +「恢复默认」入口，不白屏；单个 pane 渲染异常由 `SlotErrorBoundary` 局部降级，仍可切到其它 pane。
- **草稿**：草稿属于设置窗口会话（完整 `SettingsForm`），切 pane 不丢；关闭再打开以已保存设置为基线。
- **保存**：只有底部一个保存按钮；提交时把全部 pane 草稿合并为完整 `AppSettings` 一次提交，校验失败不提交并聚焦首个错误字段。
  成功后用返回的 `{settings, revision}` 重置外壳与所有草稿基线，关闭弹窗；失败（保存中/失败/成功三态可见）保留草稿。
- **revision 栅栏**：打开时 `settings.get` 返回 `{settings, revision}`；保存携带 `expected_revision`。
  不一致时宿主拒绝写盘并返回 `settings/revision-conflict`；前端保留草稿并提示「设置已在别处更新，请重新加载」（提供重新加载动作）。
  旧客户端不传 `expected_revision` 保持旧行为（不校验，兼容）。
- **无障碍**：Tab 顺序 = 导航 → 当前 pane 字段 → 底部保存/取消；Esc 关闭；关闭后焦点回到打开入口；axe 0 serious/critical。

## 4. 审批卡（安全关键 UI）

### 4.1 必须展示

| 字段 | 说明 |
| --- | --- |
| 动作摘要 | 一句话：要执行什么、影响哪个资源 |
| 工具/命令 | 工具名 + 完整命令/参数（不可折叠关键参数） |
| 工作目录 | 绝对路径 + 是否越出工作区 |
| 读写范围 | 将读取/写入的路径列表；网络目标（域名/端口） |
| diff 预览 | 写文件时展示变更摘要（可展开） |
| 风险级别 | low/medium/high，由工具声明 + 规则计算 |
| 超时 | 剩余时间；超时默认拒绝；可"延长一次" |
| 决策 | 允许一次 / 始终允许（本会话）/ 拒绝；默认焦点在"拒绝" |
| 记住范围 | 仅本会话、仅该工具、仅该路径；不提供全局永久允许 |

> 状态：**Phase 1 已实现子集**（`specs/changes/2026-11-01-client-approval-tool-rendering`，2026-11-01；
> 证据 `target/evidence/2026-11-01-client-approval-tool-rendering/`）。覆盖 = **7/9 行完整**（动作摘要、工具/命令、工作目录、
> 读写范围含网络目标、diff 预览、风险级别、决策）+ **1/9 行部分**（超时：剩余时间 + 超时默认拒绝；「延长一次」未做）+
> **1/9 行非目标**（记住范围选择器，含「仅该路径」scope）。**非目标（Phase 2/3）**：§4.2「危险操作二次确认」、
> §4.2「失焦通知/托盘红点/超时前 60s 通知」、「超时延长一次」、「记住范围选择器」；不提供跨重启/全局永久允许。
> 决策 wire：`allow_once | allow_session | reject | cancel`（宿主兼容旧 `approved/rejected`）；默认焦点「拒绝」、Esc=拒绝；
> 超时=`reject+timed_out`、显式取消=`cancel`、断线/窗口关闭=`reject`（唯一映射见 approval design §2.0）。
> 「始终允许（本会话）」按 `(conversation, tool)` 进程内记忆，命中时成对发 `approval_requested{auto:true} → approval_resolved{allow_session}`。

### 4.2 行为

- 审批挂起时窗口失焦 → 系统通知 + 托盘红点；10 min 超时前 60s 再通知。
- 危险操作（删除、写入工作区外、shell 网络）二次确认；快捷键不可默认确认。
- 审批决策写入 golden trace：`approval.requested → approval.resolved{decision, timed_out} → tool_call.started(可选) → tool_call.finished{status}`；
  always-allow 命中为成对 `requested{auto:true} → resolved{allow_session}`、不阻塞；策略短路（config R2）不发任何审批事件。
- 读屏：dialog role、aria-labelledby、焦点陷阱、Esc=拒绝；axe 无 serious。

## 5. 更新 UX 状态机（按 edition 分）

- 企业版：`idle → checking → up-to-date | available → downloading → verifying → ready → installing → restart-required → done | failed → rolled-back`
- 个人版：`idle → checking → up-to-date | available → 提示 + 打开 GitHub Releases 下载页`（不内置静默安装）

- 检查失败：静默重试一次，再显示"稍后重试"，不阻塞使用（强制更新除外）。
- 下载支持断点续传；校验失败自动重下（最多一次）→ 报错。
- 安装失败：回滚上一版并提示"已回滚"+ 诊断导出。
- 强制更新：Gate 页不可关闭；下载/安装进度可见；离线宽限按服务端标志执行。

## 6. 无障碍、i18n、主题（M1 前置，不得拖到 M6）

- 标准：WCAG 2.2 AA；键盘可完成全部操作；焦点可见；对比度 ≥4.5:1；**门禁以 `11` 的 serious/critical 为准**。
- 组件库：`eslint-plugin-jsx-a11y`；每个 slot 的 Story/Playwright 跑 axe；不允许 `<div onClick>`。
- i18n：`zh-CN` 默认、`en` 必需；ICU MessageFormat（复数/日期/数字）；禁止字符串拼接；缺词回退 `zh-CN` 并在开发环境报错。
- OEM 词条只覆盖 `brand.*`；缺键构建失败；`lang` 属性、locale 切换持久化、CJK 字体回退。
- RTL：M1 不做，但布局不得写死 `left/right`（用 logical properties），M6 再支持。
- 主题：token 包 + 深浅色；OEM accent；图表/代码高亮同样走 token。
- **rc.1 现状（B12 矩阵，可执行）**：`e2e/m1-a11y.spec.ts` 覆盖 client `serve`（空态/聊天/设置/命令面板/检查器），
  断言 0 serious/critical + 首 Tab 跳过链接、Tab 顺序命名/可见焦点、Esc 关闭且焦点回位、⌘K/Ctrl+B 键盘路径、
  `prefers-reduced-motion` 生效；`e2e/m6-a11y.spec.ts` 覆盖 admin 登录页 + 10 个页面 + 键盘焦点，以及静态
  `deploy/portal/index.html`（`lang`/axe/键盘）；`e2e/m5-portal.spec.ts` 覆盖 runtime portal `/portal`（axe + `lang` +
  键盘可见焦点）。门禁仍以 `11` 的 serious/critical 为准；证据
  `target/evidence/release-1.0-rc1/phase-b/b12b14/`。真实 Tauri WebView / 三平台矩阵仍属 M6 平台项（UNVERIFIED）。
- **性能 / 真实环境 harness（B14，可执行、按环境降级）**：`scripts/real-provider-smoke.sh`（真实 provider/媒体/
  embedding 在线调用；无 key 时显式打印 UNVERIFIED 并 exit 2，禁止 mock 冒充）、`scripts/load-smoke.sh`（对本机
  `serve`/server 做并发 smoke，目标 500 虚拟用户并报 p50/p95/错误率）。本机基线证据
  `target/evidence/release-1.0-rc1/phase-b/b14-perf/`、`b12b14/`；**产品容量对真实部署仍 UNVERIFIED**。

## 7. 平台集成与本地载体安全

### 7.1 Tauri capability（按窗口隔离）

| 窗口类 | capability | 允许命令 |
| --- | --- | --- |
| 主窗口 | `main` | `ipc_call`（总入口）、updater、window、dialog、store、open_url 的最小集 |
| WASM 应用窗口（可多实例） | `app` | **只允许** `app_request`；禁止 `ipc_call`/shell/fs/http |
| 登录/门禁 | `auth` | `auth_*`、`open_url`；禁止 agent 命令 |

> capability 是构建期静态文件，不能按运行期 `app_id` 生成。所有应用窗口共用 `app` capability；窗口 label 由主进程按 allowlist 生成（`app-<index>`），origin 绑定到该 app，导航只允许本 origin。

验收：应用窗口调用 `ipc_call`/`tool.shell` 必失败；`__TAURI_INTERNALS__` 不可用于业务；capability 文件进包并测。

### 7.2 本地 HTTP 载体（`serve`）

- 本地载体安全规范（Host/Origin/token/随机端口/CSP/防 rebinding）见 `21` §6；本节只定义用户可见错误：不可达时提示检查服务端/代理，不自动扫描。

### 7.3 深链 / 托盘 / 通知

- 深链 scheme 由渠道配置；只接受白名单 action（auth/app）；解析失败静默忽略并记日志。
- 托盘：打开主窗口、更新状态、退出；**关闭按钮**默认最小化到托盘，**退出应用**才结束进程（与 05 一致）。
- 通知：审批待处理、更新就绪、登录失效；不含敏感内容。

## 8. 本地数据生命周期

- 会话存储：**SQLite WAL 唯一真源**（见 `24` §7）；JSONL 仅导入/导出。
- 密钥：M1 provider key 暂存本地 JSON（0600，设置页明示）；M2 接系统密钥库；OAuth token 在企业版由安全存储/服务端处理。
- 导出：会话/设置/日志（脱敏）可导出；导出包含版本/channel 便于支持。
- 删除：清除本地数据需二次确认；企业版登出可选保留/清除缓存；不得删除服务端审计。
- 多设备：同一账号允许；本地会话不跨设备同步（除服务端内容）。

## 9. 诊断与崩溃

- 诊断页显示：版本、edition/channel、包清单与状态、最近 lifecycle 事件、最近错误链。
- 日志导出：脱敏（token/key/路径用户名）、可复制诊断 ID；崩溃自动保存 dump + 下次启动提示上传。
- 错误上报默认关（企业可配），不含 prompt/completion/文件内容。

## 10. 验收清单（M1/M2 退出条件）

可执行命令与证据路径见 `11` §2/§4/§5/§8.4；本清单是业务断言。

- [ ] **M1** 个人版：首装→配 key（设置）→首轮对话→工具→重启恢复→工作区隔离，e2e；审批/引导/更新回滚为 M2。
- [ ] 企业版 OEM：免填地址登录→bootstrap→对话→强制更新→回滚，全流程 e2e。
- [ ] 企业版官方：首配地址→登录→修改地址需重登，e2e。
- [ ] M1 页面六态 + 键盘/axe；审批卡读屏/超时通知为 M2。
- [ ] 应用窗口 capability 隔离测试通过；`serve` Host/Origin/rebinding 测试通过。
- [ ] i18n 缺词/复数/日期；OEM 覆盖缺键构建失败。

## 调度/自动化设置

- 设置页“调度/自动化”：开关 `enabled` + 扫描间隔 `scan_seconds`（秒，5..=3600）。
- 优先级：显式保存的设置（包括默认 `disabled/10`）> 无设置文件时的 `ZEOKEEL_ENABLE_SCHEDULE_AUTORUN` env fallback。
- 保存后热生效：执行器常驻内核任务，disabled 时 idle；开关/间隔更新无需重启。UI 当前不展示 env fallback 的 effective 状态（fresh install 仍按 env 运行；用户保存后以设置为准）。

## 网络搜索设置（R9.3/B11）

- 设置页“网络搜索”：端点 `base_url`（**仅 https**，可留空）+ 请求超时 `timeout_ms`（ms，1000..=120000，默认 15000）。
- 优先级：显式保存的设置 > `ZEOKEEL_SEARCH_URL` env fallback > 内置 DuckDuckGo 默认端点；空 `base_url` 表示“不覆盖”（env/默认生效），超时无 env fallback（设置或 15000）。
- 生效方式：桌面在 `settings.set`/启动时发布非密 `RuntimeSettings.web_search`；`tool-web-search` 在每次调用时解析（无需重启），不回显、不落任何搜索凭据（该端点无密钥）。
- 校验：非 https/非法 URL、超时越界由 `settings.set` 拒绝且不保存；UI 同步给出字段错误。
- 版本：工具出口端点不是 AI provider，C7（企业 AI 只走网关）不适用；C6 弱管控不限制工具出口，故 personal/enterprise 均可配置。

## 11. 交互契约与交互 ID（强制）

> 本节是**交互契约内容与 ID 规则**的唯一权威；证据/门禁口径见 [`11`](11-testing-and-evidence.md) §8.6，交互模式与视觉/ARIA 规则见 [`27`](27-ui-design-system.md) §2.5/§2.6，此处不复述。

任何客户端 surface（页面/面板/弹窗/菜单/输入区）新增或修改交互行为，变更目录必须包含 `specs/changes/<id>/interaction.md`（模板 `specs/templates/interaction.md`；`scripts/new-change.sh` 自动生成）。

**必填字段**：表面/入口、归属包、权威文档、`状态：active | planned`、证据根、测试落点、元素与语义（role/accessible name/键盘/默认焦点）、状态模型、交互条目表、视觉/ARIA 基线、预算与一致性约束。

- **ID 规则**：`<SURFACE>-<AREA>-<NN>`（大写；AREA 如 `KEY`/`FOCUS`/`IME`/`CMD`/`AT`/`FILE`/`ATT`/`ERR`/`A11Y`/`VIS`/`PERSIST`）。ID 在仓库内唯一；一个 ID 至少一个测试，一个测试可覆盖多个 ID，但不得引用未定义 ID。
- **优先级**：P0 = 主路径/焦点/数据一致性；P1 = 高频细节/可达性/回归基线；P2 = 可选增强（不进门禁，契约可不列）。P0/P1 必须写测试落点与证据路径。
- **状态模型**：至少覆盖六态（空/加载/错误/离线/流式中/已取消）+ `busy`/`disabled`/`composing`/`menu-open`（按 surface 适用）。
- **active / planned**：`active` 参与门禁；`planned` 只 WARN，实现开始时改 `active` 并补 `@interaction:<ID>` 测试与证据。
- **基线 surface（已实现，`active`）**：composer、settings dialog、inspector、command palette、sessions sidebar——共 19 个 ID（P0 = 11），见 `specs/changes/2026-11-01-ui-interaction-verification/interaction.md`。
- **在途 surface（`planned`）**：Phase 2a `2026-11-01-client-composer-surface`、Phase 2b `2026-11-01-client-side-panels` 与其子 spec `2026-11-01-client-activity-panels` 各自 `interaction.md` 把其 R1–R8 拆成 ID；企业版登录/网关/商店/管理台 surface 后续按同一模板补。
- **一致性**：只能复用 `27` §2.5 的唯一交互模式清单；禁止第二套 focus/dialog/menu/toast（C17）。

### 11.1 直觉检查（INT-*，强制）

> 本节定义**契约必填的两个直觉文件与映射要求**；直觉原则/判据/反例只在 [`27`](27-ui-design-system.md) §2.7 定义，
> 认知走查与用户测试方法只在 [`27`](27-ui-design-system.md) §2.8 定义，证据/门禁口径见 [`11`](11-testing-and-evidence.md) §8.6。本节不复述原则表。

UI 变更在 `interaction.md` 之外，还必须交付两个同目录文件（模板：`specs/templates/intuition.md`、`specs/templates/intuition-walkthrough.md`；`scripts/new-change.sh` 自动复制）：

- **`intuition.md`（第七件）**：`状态：active | planned`；`active` 时 **≥3 条 INT 行**，且该 surface 的**每个 P0 交互 ID 至少映射 1 条 INT**；每行含场景/top task、用户直觉期望（可观察）、反例、验证方式、证据。
- **`intuition-walkthrough.md`（第八件）**：3 persona × 3 top task，逐步回答 4 问（知道目标？看得到控件？能预判反馈？能恢复？）+ 摩擦表（INT ID、P0/P1/P2、最小修法、状态）；P0 摩擦未解决不得进入实现/发布。
- **planned 态**：`planned` 契约两个文件可选（门禁只 WARN）；实现开始时必须补齐并把 `interaction.md` 改 `active`，否则不进入实现/验收。
- **验证方式**：`e2e proxy`（可观察行为代理）/ `review`（fresh-context 3-persona 走查）/ `user test`（≥5 名真实用户 think-aloud，发布前；未执行标 UNVERIFIED）。**e2e 不能证明直觉**；质量由 reviewer 走查 + 用户测试判定，门禁只查存在性/最低行数。
- **证据**：走查结论进评审记录/`acceptance.md` 并引用 INT ID；用户测试证据 `target/evidence/<change>/usability/`（匿名化，未跑标 UNVERIFIED）。任何"符合直觉"结论在 ≥5 名真实用户测试前不得声称。

