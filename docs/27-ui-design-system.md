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

## 3. UI 包组合（一切皆包）

前端组件是包；app 是薄壳。**shell 只认 registry 的 slot/component，不直接 import 具体 UI 包。**

| 包 | 角色 | 内容 | 依赖 |
| --- | --- | --- | --- |
| `@zeokeel/ui-kit` | `ui.kit` | DTO/类型、`UiProvider/useUi` context、BrandLogo、icons、Button/Switch/StatusDot/Chip | react（M1）；Radix tooltip 目前在 shell，T21 迁入 |
| `@zeokeel/ui-sessions` | `ui.slot.sessions` | Sidebar（会话列表/新建/选中/设置入口） | ui-kit |
| `@zeokeel/ui-chat` | `ui.slot.chat` | ChatView + assistant-ui Thread/Composer/ActionBar/工具卡/审批/空态 + ExternalStoreRuntime adapter | ui-kit + `@assistant-ui/*` |
| `@zeokeel/ui-settings` | `ui.slot.settings` | SettingsDialog + 表单校验 | ui-kit + Radix dialog |
| `@zeokeel/ui-shell` | `ui.shell` | 应用框架骨架 | ui-kit |
| `@zeokeel/ui-inspector` | `ui.slot.inspector` | 右栏文件/变更/终端骨架 | ui-kit |
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
  { "brand": { "name": "ZeoKeel", "powered_by": "show",
               "logo": { "light": "logo.svg", "dark": "logo-dark.svg" } } }
  ```
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
