# 27 · UI 设计系统、UI 包组合与品牌资源

> 本文是 **客户端视觉/组件/UI 包组合/渠道品牌资源** 的唯一权威。任何 UI 变更先改本文（或 16/19/23 对应节），
> 经 `docs/11` §9 的文档评审流程后，再写 spec/代码。实现目前对应 M1 个人版桌面客户端。

## 1. 设计语言（采用 assistant-ui 官方 design system）

参考 <https://www.assistant-ui.com/design>（"Every surface is a printed page"）。我们**采用其规则，不复制其品牌色**：

| 规则 | 采用 |
| --- | --- |
| 中性色单一 hue | `--tint: 250`（冷灰蓝；官方 106 是沙色，我们用更清新的冷调） |
| 纸/墨/静默/发丝线 | light：bg `oklch(0.992 0.003 250)`、fg `oklch(0.145 0.008 250)`、muted `oklch(0.968 0.005 250)`、border `oklch(0.922 0.006 250)`；dark 同 hue 0.17/0.96/0.22/0.28 |
| 唯一 live accent | `blue-500 oklch(0.623 0.214 259.815)` 只表示"流式中/运行中/已选中/连接中"；无 live 状态页面无蓝 |
| 错误 | destructive 只表错误，不是第二强调色；不使用第二个品牌色做按钮；焦点环用 foreground/muted，不用 live blue（blue 只表示 live） |
| 品牌色 | 只存在于**官方 logo 图片**（渠道资产）；禁止 UI 渐变/glow/纹理仿品牌 |
| 圆角 | page 0 / document 6 / control 8 / surface 10 / xl 12 / thread 16 / capsule 9999 |
| 阴影 | 普通表面零阴影；只有浮层（dialog/toast/popover/drawer）可 lift；禁止 border+ring+shadow 叠同一边；composer 聚焦只改边框色，不加 blue ring |
| 线 | 只保留 section 边界 hairline；行内用间距+hover 填充，不做 ledger 式全边框 |
| 字体 | Public Sans（UI/标题）+ JetBrains Mono（代码/命令/标识），`@fontsource/*` 本地打包、无 CDN；中文回退 PingFang SC/Noto Sans SC/system-ui；**不使用衬线**；正文行高 1.6 |
| 标题层级 | 字号/字重建立层级（空态 32/500、页面标题 20/600、正文 14/400，行高 1.6）；标签 12px 句首大小写、正常字距 |
| 动效 | 120–200ms，只解释状态变化；`prefers-reduced-motion` 全部关闭；禁止夸张动画 |

**禁用**：暖米色+衬线"老气"方案、紫蓝 SaaS 渐变、emoji 当图标、自绘品牌 mark、外部字体/CDN。

## 1.5 安全/密钥 M1 例外

- 个人版 provider/API key 在 **M1 暂存本地 JSON（0600）**，设置页明示；**M2 接系统密钥库**（keyring），
  文档/验收按 M1 例外说明，不宣称已用 keychain。企业版客户端不得持有上游 key（C7/ADR-031）。
- 工作区仍不是安全沙箱（`26`）；UI 不得把"工作区根"表述成权限边界。

## 2. 组件与交互基线

- 采用 assistant-ui primitives（Thread/Message/Composer/ActionBar/Suggestion/Markdown），不自己写聊天层/Markdown/焦点管理。
- 消息区最大宽 760px 居中；侧栏 248px（M1 高密度；旧 288 已废弃）；用户消息右对齐 `thread` 圆角 + muted 底；助手无头像/无气泡，靠留白与内容层级区分。
- hover/focus 才显示操作条（复制/重发）；流式光标 blue-500；工具卡 hairline + 状态点（running 蓝/done 中性/error destructive）；审批卡用 data part 占位，后续接 tool-approval 协议。
- **六态**：空/加载/错误/离线/流式中/已取消，每个交互组件必须覆盖；离线禁用发送并显示横幅；取消保留已生成内容并标记。
- **可访问性**：键盘全流程、焦点可见、aria-label、对比度 ≥4.5、axe serious/critical = 0；i18n zh-CN/en-US，`<html lang>`/`data-theme` 同步。
- **窄窗** ≤900px：侧栏抽屉，workspace/session 选择在抽屉顶部；抽屉可用 Esc 关闭。
- 真实后端（serve 载体）与 `?mock=1` 均须能操作；mock 只用于本地演示/截图，不作为验收替代。

### 2.5 交互模式清单与一致性规则

同一交互只允许**一套**实现；以下模式必须用 `@zeokeel/ui-kit` / Radix 原语（现状例外：Radix Tooltip 仍在 shell，T21 迁入，见 §3 规则 4）。新增模式先登记本节再实现。

| 模式 | 唯一实现 | 禁止 |
| --- | --- | --- |
| Dialog | ui-kit + Radix Dialog | 手写 focus trap/Esc 层叠/遮罩 |
| Menu | ui-kit + Radix DropdownMenu | 自绘下拉/键盘分叉 |
| Popover | ui-kit + Radix Popover | 定位/关闭逻辑各写一份 |
| Tooltip | ui-kit（T21 迁入前只允许 shell 唯一实例） | 每个包各包一层 |
| Toast | ui-kit toast（app 单一出口） | 第二套错误提示系统 |
| Empty / Error | ui-kit 空态/错误态组件 + i18n | 页面内联自绘占位 |
| InlineEdit | 单行编辑（输入框 + 确认/取消） | 双击直接改状态 |
| Confirm | ui-kit ConfirmDialog（危险操作二次确认） | 用普通 Dialog 冒充确认 |
| Loading | ui-kit Spinner/Skeleton（延迟出现，避免闪烁） | 散落的自制 loading |

- **token-only**：颜色/圆角/间距/动效只用 CSS vars；禁止 raw hex / magic px / 第二套阴影。
- **六态**：空/加载/错误/离线/流式中/已取消；每个交互组件缺一即不通过评审。
- **焦点与键盘**：焦点可见、Tab 顺序符合阅读序；Esc 只关闭最上层（层叠按打开顺序）；IME composing 期间 Enter/`/`/`@` 不触发命令；关闭后焦点回打开入口。
- **错误分层**：inline（字段/行内）> banner（区块级/离线）> toast（瞬时）> modal（阻塞性确认）；同一错误只出现一层。
- **动效**：120–200ms，只解释状态变化；`prefers-reduced-motion` 全部关闭。
- **i18n**：所有文案来自字典（zh-CN/en-US），禁止硬编码；`<html lang>` 与 locale 同步。
- **响应式**：375（窄窗抽屉）/768（右栏自动收起、设置导航退化）/1280（桌面基线）三档；布局用 logical properties，不写死 left/right。
- **hover-only 必须有等价 focus/键盘路径**：hover 才出现的操作条/菜单，键盘 focus 与显式菜单入口必须可达；不允许"只有鼠标能做"。

### 2.6 视觉与 ARIA 基线

- `toMatchAriaSnapshot`：每个 surface 至少一条结构/角色/状态基线；动态项用正则/部分模板，不做整页全量快照。
- `toHaveScreenshot`：只用于稳定小区域（当前仅 composer 空态，light/1280）；禁动画、隐藏光标、`maxDiffPixelRatio ≤ 0.002`；动态区域加 mask，不截图含时间/随机 id 的区域。
- axe：每个 surface 0 serious/critical；失败输出违规 id 列表（浏览器/键盘测试见 `11` §8.6）。
- 基线更新只能由人确认后 `pnpm e2e:ui:update`；CI/agent 不得自动接受（命令与门禁见 `11` §8.6）。
- 深色主题与 375/768 档以 axe + 结构断言为主，像素基线后置；跨平台渲染差异登记 `KNOWN-LIMITATIONS.md`。

### 2.7 直觉交互判定（INT-*）

> 本节是**"真人是否觉得直观"的判定 SSOT**：原则、判据、反例、验证方式只在这里定义；每次变更的 `intuition.md`
> 只填本 surface 的场景与证据（`docs/19` §11.1），不得复制本节原则表。

- `INT-01..12` 在本节定义一次；各 surface 的 `intuition.md` **引用**这些 ID，不自造同名新编号；新增原则才扩编号。ID 前缀 `INT-`；每条必须有**可观察判据**与**反例**，验证方式 ∈ {`e2e proxy`（可观察行为代理）、`review`（专家走查）、`user test`（≥5 名真实用户）}。
- **e2e 只能做代理，不能证明直觉**：焦点/Esc/busy/文案键/空态可被脚本断言，但"用户是否知道下一步/是否犹豫/是否一眼看懂"只能由走查与用户测试判定。
- 判据用于评审与走查记录，不作为 CI 自动断言；门禁只查产物存在性（`docs/11` §8.6）。

| ID | 原则 | 判据（可观察） | 反例 | 验证方式 |
| --- | --- | --- | --- | --- |
| INT-01 | 可发现性 | 主操作/入口在不 hover、不培训时可见，或由空态/首次提示给出快捷键与下一步 | 发送按钮仅 hover 出现；⌘K 无提示且无可见入口 | e2e proxy + review |
| INT-02 | 预期一致 | 键盘/手势/文案符合平台与产品惯例（Enter 发送、Shift+Enter 换行、Esc 关最上层、Tab 顺序符合阅读序） | Esc 在弹窗内无反应；Shift+Enter 触发发送 | e2e proxy |
| INT-03 | 状态可见 | 每个用户动作 ≤100ms 出现 busy/成功/失败/流式可见反馈，重复点击被抑制 | 点击后无反馈，用户重复提交 | e2e proxy + review |
| INT-04 | 就近反馈 | 字段/行内错误出现在触发点旁并给可修文案；不只弹全局 toast | 只显示"保存失败"，不指出哪个字段 | e2e proxy + review |
| INT-05 | 默认安全 | 危险/不可逆操作需确认或可撤销；默认焦点不在破坏性按钮；不预填危险值 | 回车直接删除会话/覆盖设置 | e2e proxy + review |
| INT-06 | 可恢复 | 失败/关闭/取消保留草稿与已生成内容，提供重试/撤销/恢复入口 | 发送失败后输入框被清空 | e2e proxy + review |
| INT-07 | 认知负担 | 单屏主要选择 ≤7 项；分组/渐进披露/合理默认；识别优于回忆 | 40 个字段一屏堆叠、需记住上一步输入 | review（Hick/Miller） |
| INT-08 | 文案即引导 | 空态/错误/toast 动词开头、说明下一步与影响；无内部术语/错误码直出 | "操作失败（code 500）" | review + i18n 检查 |
| INT-09 | 键盘/焦点 | 键盘可完成主任务；焦点可见；关闭/确认后焦点回打开入口；输入框不吞快捷键 | 只能鼠标保存；Esc 后焦点丢失 | e2e proxy |
| INT-10 | 空态/错误态 | 空态给下一步；错误给原因+修法+重试；不白屏、不假成功 | 空白页或只有一个红叉 | e2e proxy + review |
| INT-11 | 感知性能 | 输入反馈 <100ms；面板切换/加载 ≤400ms 或显示骨架/busy；避免闪烁 | 输入卡顿、切面板整页白屏 | e2e proxy/perf + review |
| INT-12 | 单一模式 | 同一交互（Dialog/Menu/Toast/焦点/确认）只有一套实现与一致行为（§2.5、C17） | 两个弹窗 Esc 行为不同 | review |

### 2.8 认知走查与用户测试

> 每个 UI 变更（`active`）必须交付 `intuition-walkthrough.md`：3 persona × 3 top task × 每步 4 问 + 摩擦表；
> 模板 `specs/templates/intuition-walkthrough.md`，产物与 `interaction.md` 同目录（`docs/19` §11.1）。走查是**专家走查**，不等于用户测试。

**3 persona**：

| ID | Persona | 特征 | 关注点 |
| --- | --- | --- | --- |
| P1 | 首次使用者 | 不懂代码、第一次用 | 能否不培训完成首任务；入口是否可见 |
| P2 | 高频键盘用户 | 每天使用、键盘流 | Enter/Esc/Tab/⌘ 是否符合预期；焦点是否丢 |
| P3 | 出错/辅助技术用户 | 键盘+读屏、离线/权限失败 | 错误可读可恢复；焦点/aria 正确 |

**3 top task**（按频率 × 失败代价选取；每次变更按本 surface 重选）：

| ID | Task | 频率 | 失败代价 |
| --- | --- | --- | --- |
| T1 | 首次完成主任务（如发送一条消息） | 极高 | 高 |
| T2 | 修改并保存设置（或该 surface 的核心配置动作） | 中 | 高 |
| T3 | 键盘/命令路径组合任务（如 ⌘K + 打开面板） | 高 | 中 |

**每步 4 问**：① 用户知道目标吗？② 看得到可操作控件吗？③ 能预判操作反馈吗？④ 出错能恢复吗？
逐步记录"通过/卡点"，卡点引用 INT ID 进摩擦表：

| INT | 严重度 | 描述 | 最小修法 | 状态 |
| --- | --- | --- | --- | --- |
| INT-xx | P0/P1/P2 | 卡点 | 修法 | open/closed |

- **P0 摩擦未解决 → 不得进入实现/发布**；无 P0 时必须显式写"无 P0 摩擦"。
- 评审者（fresh context、非实现者）按本节走查并核对 `intuition.md` 覆盖；只写"看起来不错"不合格（`docs/11` §9）。

**用户测试协议（发布前）**：

- 对象：**≥5 名真实用户**（非实现者，至少 2 名不熟悉本产品）；<5 人不作为发布证据。
- 方法：think-aloud、不提示操作路径；每人跑 T1–T3。
- 指标：任务成功率、首次点击正确率、完成时间、错误数、SEQ/SUS。
- 证据：`target/evidence/<change>/usability/{script.md,notes/*.md,scores.json}`（匿名化；target gitignore）。
- **未执行 → 标 UNVERIFIED**；任何"符合直觉/直观"结论在 ≥5 名真实用户测试完成前不得声称（`docs/11` §8.6、`KNOWN-LIMITATIONS.md` §3.4）。
- 用户测试不接入 `pnpm e2e:ui`/CI（需要真人，不可复现为自动断言）。

## 3. UI 包组合（一切皆包）

前端组件是包；app 是薄壳。**shell 只认 registry 的 slot/component，不直接 import 具体 UI 包。**

| 包 | 角色 | 内容 | 依赖 |
| --- | --- | --- | --- |
| `@zeokeel/ui-kit` | `ui.kit` | DTO/类型、`UiProvider/useUi` context、BrandLogo、icons、Button/Switch/StatusDot/Chip | react（M1）；Radix tooltip 目前在 shell，T21 迁入 |
| `@zeokeel/ui-sessions` | `ui.slot.sessions` | Sidebar（会话列表/新建/选中/设置入口） | ui-kit |
| `@zeokeel/ui-chat` | `ui.slot.chat` | ChatView + assistant-ui Thread/Composer/ActionBar/工具卡/审批/空态 + ExternalStoreRuntime adapter | ui-kit + `@assistant-ui/*` |
| `@zeokeel/ui-settings` | `ui.slot.settings` | SettingsDialog + 表单校验 | ui-kit + Radix dialog |
| `@zeokeel/ui-shell` | `ui.shell` | 应用框架骨架 | ui-kit |
| `@zeokeel/ui-inspector` | `ui.slot.inspector` | 右栏文件/变更/终端；变更 tab 调 `workspace.changes`（git status 只读、空/加载/错误+重试）；变更项为**文件级** +add/−del 计数（无行级 diff），文案标注“文件变更” | ui-kit |
| `@zeokeel/ui-commands` | `ui.slot.commands` | ⌘K 命令面板骨架 | ui-kit |
| `apps/desktop/web` | 薄壳 | transport/controller/reducer/i18n/theme/UiProvider/registry 槽位/boot/error/header/toast | registry 生成的包 |

规则：
1. 包在 `zk.toml` 用 `[[ui.contributions]]` 声明 `slot/component/order`；`zk compose` 生成 `apps/desktop/web/src/generated/registry.ts`
   （`SessionsSlot/ChatSlot/SettingsSlot` + `COMPONENTS` + `shared/contracts/ui-contributions.json`）。
2. 包之间只允许依赖 `@zeokeel/ui-kit`；业务包不得被其它包 import；app 的业务 slot 只从 registry 取组件（基础件可直接用 ui-kit）。
3. `@assistant-ui/*` 只允许出现在 `ui-chat`；React 为 peer dependency。M1 的 app 可直接使用 `ui-kit` 基础件（BrandLogo/icons），业务 slot 必须经 registry。
4. 共享视觉/交互进入 `ui-kit`；禁止在多个包复制同一组件（C17）。M1 例外：Radix Tooltip 仍在 `App.tsx`，T21 迁入 ui-kit。
5. Tailwind 4：`apps/desktop/web/src/app.css` 必须为每个 UI 包源码加 `@source`；新增 UI 包必须同步加。
6. 换包 = 改 edition/profile → `zk compose` → 重新构建；运行期不加载/不切换 UI 包（C2）。
7. 包级独立 typecheck/conformance（T21）后置；当前由 app tsc + Rust conformance 覆盖。

## 4. 品牌与 logo（渠道资产，构建期注入）

- **权威来源**：渠道仓库的 logo 文件。`official` 渠道 = `brands/official/logo.svg`（浅色）+ `logo-dark.svg`（深色），
  官方渠道必须使用这两个文件；个人版使用官方 logo。
- `brands/<channel>/channel.json` 必填：
  ```json
  { "identity": { "slug": "zeokeel", "product_name": "ZeoKeel", "app_id": "dev.zeokeel.app",
                  "company": "ZeoKeel", "copyright": "ZeoKeel", "deep_link_scheme": "zeokeel",
                  "app_origin_scheme": "zk-app", "data_dir_name": "ZeoKeel", "product_home_dir": ".zeokeel" },
    "brand": { "powered_by": "show", "logo": { "light": "logo.svg", "dark": "logo-dark.svg" } },
    "assets": { "accent": "#0F766E" }, "defaults": { "server_url": "" } }
  ```
  完整字段/校验以 `09` §3 为唯一真源；`product_name` 是显示名，`identity.*` 负责安装包/路径/协议身份。
- 构建期：`zk compose` 校验 logo 存在 → 复制到 `apps/desktop/web/public/brand/logo.svg`、`logo-dark.svg`，
  并生成 `apps/desktop/web/src/generated/brand.ts`（`BRAND.logoLight/logoDark`）与 `shared/contracts/brand.json`；
  这些是 `@generated`，禁止手改；`--check` 必须绿。
- UI 用 `BrandLogo` 按解析主题切换 light/dark；**本仓库不得出现其它 logo 产物**；需要时只能从这两个官方 logo 派生
  （例如 Tauri 应用图标由 `logo.svg` 渲染为 `apps/desktop/icons/icon.png`）。
- 其它渠道可配置自己的 logo（同 schema）；不得让 UI 硬编码官方素材路径或内置官方 logo 副本。
- **安全**：`brand.logo.*` 必须是渠道目录内相对路径；`zk compose` 做 canonicalize → 必须仍在渠道目录内、拒绝 `..`/绝对路径/符号链接逃逸、
  限制 256KB、拒绝 `<script>`/外链资源；`brand.powered_by` 必须是 `show|hide` 枚举；路径/内容不合法 = 构建失败。
- `apps/desktop/web/public/brand/*.svg` 是官方源文件的逐字节复制（不加头、不改内容）；`brand.ts` 有 `@generated`；
  `--check` 同时比对源/副本字节。应用图标由 `scripts/gen-icons.sh` 从官方 logo 渲染，禁止手绘。
- OEM 的 `powered_by`/文案仍按 `23`/ADR-056；品牌资源只经渠道注入，不在代码里按渠道分支。

## 5. 变更与评审（强制）

任何"发现的可用方案"（设计系统、库、DSH 模式、颜色、logo 规则等）必须：

1. **先写文档**：落到本文/`16`/`19`/`23`/`26` 或新 ADR；记录来源、取舍、失败/回退条件（`11` §9）。
2. **文档评审**：由非实现者（子代理/ teammate）只读评审，输出 P0/P1 + 结论；修复后复评通过。
3. **再写 spec/施工**：C16 五件套 `approved` 后按 spec 实现；UI 变更附截图/axe/键盘证据。
4. **证据**：`target/evidence/` 或 `apps/desktop/web/screenshots/`（light/dark/narrow、六态、真实 serve 流式）+ `screenshots/axe.json`；
   e2e 用 Playwright 模拟真人操作（非仅组件测试）。
5. 未评审/未落文档的"好看方案"不得直接进代码；发现文档与实现冲突时先改文档再改实现。

## 6. 参考

- assistant-ui design：<https://www.assistant-ui.com/design>；Claude clone：<https://www.assistant-ui.com/examples/claude>；
  Radix Colors：<https://www.radix-ui.com/colors>（可访问色阶）。
- 本文只引用这些作为设计事实/规则来源；不引入其代码依赖（除已声明 npm 包）。
