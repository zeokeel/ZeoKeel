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
| `CLIENT_TOO_OLD` / 426 | `ui.gate.update` | 下载更新、重试、查看版本说明；不进入聊天 |
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
| 首配服务端地址（官方企业版） | `ui.first-run` | `client-auth` | 地址非法、不可达、证书错误、保存成功 |
| 主界面（M1: ui.shell + 三内容槽） | — | `@zeokeel/ui-shell` + app 薄壳 | 正常、离线、连接中、断线重连中 |
| 工作区切换/首用 | `ui.sessions` | `ui-sessions` | 无工作区、加载中、目录不可用、切换/取消流式 |
| 对话 | `ui.chat` | `ui-chat` | 六态 + 长会话 + 取消/重试 |
| 输入 | `ui.composer` | `ui-composer` | 空、 composing（IME）、附件中、超长、发送失败 |
| 工具调用 | `ui.toolcall` | `ui-tools` | pending/running/success/failed/cancelled |
| 审批 | `ui.approval` | `ui-approval` | 等待、超时、批准/拒绝、记住选择 |
| 设置 | `ui.settings` | `ui-settings` | 保存中、失败、需重启、需重登 |
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

### 4.2 行为

- 审批挂起时窗口失焦 → 系统通知 + 托盘红点；10 min 超时前 60s 再通知。
- 危险操作（删除、写入工作区外、shell 网络）二次确认；快捷键不可默认确认。
- 审批决策写入 golden trace：`approval.requested → approved/rejected/timeout → tool.result`。
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
