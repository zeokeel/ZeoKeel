# 2026-10-30 · UI 设计系统与 UI 包/品牌资源文档评审

> C19 / `docs/11` §9 文档评审记录。评审对象是新写的 UI/品牌规范 + 当前 M1 实现对照。
> 评审者：`audit-runtime`（非实现者，只读）。
> 结论：**CHANGES-REQUESTED**（P0=5，P1=8，P2=12）。

## 0. 范围与快照

- 文档：`docs/27-ui-design-system.md`（新）、`docs/16-ui-packages.md`、`docs/19-client-ux-flows.md`、
  `docs/23-support-oem.md`（logo 节）、`docs/24-composition-and-runtime-spec.md` §3、`docs/README.md` §1/§2、
  `docs/09-delivery-and-oem.md` §3（渠道 schema）、`docs/11-testing-and-evidence.md` §8/§9、`AGENTS.md` §3.1/C19。
- 实现对照：`brands/official/{channel.json,logo.svg,logo-dark.svg}`、`tools/zk/src/main.rs`、
  `apps/desktop/web/src/generated/{brand.ts,registry.ts}`、`apps/desktop/web/src/{app.css,App.tsx,theme.tsx,i18n.tsx}`、
  `apps/desktop/web/src/state/*`、`packages/ui/*/web/src`、`apps/desktop/icons/icon.png`、
  `apps/desktop/web/screenshots/*.png`、`apps/desktop/web/screenshots/axe.json`、`apps/desktop/web/tests/*`。
- 快照：HEAD `f48b87f`，工作区为 dirty（多份 docs 与 `apps/desktop/web/**` 有未提交修改，task-11/12/14/15 仍在进行）；
  本评审针对 2026-10-30 17:30 左右的工作区。若后续文件变动需复评。
- 只读命令：`rg`/`read`/`read_image`；`python3 scripts/docs-check.py` → 0 errors/0 warnings/32 files；
  `python3 scripts/spec-check.py` → 0/0。门禁绿只代表结构与链接合法，不代表文档与实现一致。

## 1. 结论摘要

- **做得好**：`27` 的 token 数值与 `app.css` 基本逐项一致（tint 250、light/dark 灰阶、blue-500、圆角 scale、字体本地打包）；
  UI 包边界大体成立（app 只从 registry 取 slot；业务包只依赖 ui-kit；`@assistant-ui/*` 只在 ui-chat；
  app 内展示副本已迁出，无第二份实现）；官方 logo 由渠道构建期注入，BrandLogo 是唯一渲染入口，未发现 C15 参考仓库依赖。
- **不能 APPROVED 的原因**：文档描述的不是同一套系统——
  `16` 仍是 target 模型（ui-shell-kit/ui-shell/transport、`registry.gen.ts`+vite virtual、SLOT_DEFS/闭包校验、
  `zeokeel.client` 清单），M1 实际是 `27` §3 的四包 + `registry.ts` + `ui.sessions` slot；
  `09` 的渠道 schema 与 `27/23`/实现冲突；品牌资产拷贝存在路径安全缺口；Tauri 图标声称"由 logo 派生"但无生成器/所有权；
  文档承诺的 drawer Esc/workspace/审批 a11y 与六态/真实后端证据未落地。

## 2. P0（阻塞：矛盾 / 安全 / SSOT / C11）

### P0-1 文档 16 与 M1 实现不是同一套 UI 包/槽位模型
- **证据**
  - `docs/16-ui-packages.md:4` 结论先行："前端包 = pnpm workspace 成员包（带 `zeokeel.client` 清单）…"；
    `docs/16-ui-packages.md:52-68` 给出"每个 UI facet 的 `package.json` 必须带清单"及 `zeokeel.client` 示例；
    但同文件 `docs/16-ui-packages.md:77-80` 的 M1 注记说 "`zeokeel.client` … 不在 M1 使用；M1 不并存两套声明"。
    实际 `packages/ui/*/web/package.json` 均无 `zeokeel.client`（只有 name/version/main/types/deps）。
  - `docs/16-ui-packages.md:28-30` 仍列 `ui/shell-kit/`、`ui/shell/`、transport 包，并给它们角色；
    实际 `packages/ui/` 只有 `ui-kit`、`ui-sessions`、`ui-chat`、`ui-settings`（`find packages/ui`）；
    根 `zk.toml` 与 `editions/personal.toml` 也没有 `ui.shell-kit`/`ui.shell`/`ui.transport` 角色。
  - `docs/16-ui-packages.md:99-121` 的 `SLOT_DEFS` 无 `ui.sessions`；而 `packages/ui/ui-sessions/zk.toml` 的
    `[[ui.contributions]] slot = "ui.sessions"`，`tools/zk/src/main.rs:926-928` 也用 `ui.sessions` 生成 `SessionsSlot`。
    按 `16:188`"slot 在 SLOT_DEFS 里存在"的规则，当前组合应失败，但 compose 没有该校验。
  - `docs/16-ui-packages.md:133-139` 声明 composer 构建期"四件事"（闭包校验、slot 存在、props_schema 兼容、生成 registry.gen.ts）；
    实际 `tools/zk/src/main.rs:935-988` 只按硬编码 `slot_binding` 生成 import/绑定，`ui_contributions_json` 不含 `props_schema`，
    无 component/slot 闭包校验；生成的 `apps/desktop/web/src/generated/registry.ts` 是 `registry.ts`（无 `.gen` 后缀），
    `App.tsx:12` 直接 import，不存在 `virtual:@zeokeel/registry`（`vite.config.ts` 无相关插件）。
  - `docs/27-ui-design-system.md:40-56` 已按 M1 实际（四包 + `registry.ts`）描述，并与 `16` 的 target 描述冲突。
- **影响**：`16` 是 UI 包/槽位 SSOT，当前无法据此实现 compose/registry，也无法据此判断"换包是否成立"；C4/C5/C11 语义落空。
- **修法**：把 `16` 拆成"M1 现状（以 27 §3 为准）"与"target 扩展（T21 后置）"两段；删除/标注 §2 的 `zeokeel.client`
  示例与 §3 的 shell-kit/shell/transport 目录；把 `ui.sessions` 补进 SLOT_DEFS 或改用已有 slot；明确
  "闭包校验/props_schema/registry.gen.ts/virtual 模块"是 target 而非 M1，或直接实现后再写为现状。

### P0-2 渠道品牌资产拷贝路径不安全（安全）
- **证据**：`tools/zk/src/main.rs:484-500` 只校验 `brand.name` 与 logo 非空/存在，`dir.join(logo)` 未做
  canonicalize、相对路径、`..`、绝对路径、符号链接检查；`tools/zk/src/main.rs:709-717` 直接把该文件内容
  复制为 `apps/desktop/web/public/brand/logo.svg`。`brands/<channel>` 来自 CLI `--channel`（同上 `load_channel` 直接 `root.join("brands").join(channel)`）。
  `docs/27-ui-design-system.md:63-72` 只说"其它渠道可配置自己的 logo（同 schema）"，无路径/类型/大小限制。
- **影响**：OEM 渠道仓（客户/我方子仓）可通过 `"logo": { "light": "../../../etc/passwd" }` 把构建机任意文件
  打进客户端静态资源；也可放大文件/塞入带 `<script>` 的 SVG（WebView 同源静态资源，存在 XSS/资源耗尽面）。
  `docs/09:118` 只写了"SVG/尺寸/ICC"校验，但 `zk` 未实现。
- **修法**：`load_channel` 对 channel id 做 `^[a-z0-9][a-z0-9-]{0,31}$`；对 logo 路径 canonicalize 后必须
  starts_with(channel_dir)，拒绝绝对路径/`..`/symlink；只允许 `.svg`、大小上限（如 ≤512 KiB）、拒绝外部 `href`/`<script>`；
  在 `docs/27` §4/`docs/23` 写清校验项，并加负向渠道 fixture 测试（T0/T1）。

### P0-3 渠道/品牌 schema 的 SSOT 冲突：`09` vs `27/23`/实现
- **证据**：`docs/09-delivery-and-oem.md:118-119` 的渠道资产是 `assets.logo_primary`/`logo_dark`（"SVG 平坦 hex 方块 + 方块外 mark"）
  与 `assets.app_icon`（PNG 1024²）；`docs/09:122` 的 `brand.powered_by` 要求 `show`/`hide`、"未知值构建红"。
  但 `docs/27-ui-design-system.md:63-68`、`docs/23-support-oem.md:126-132`、`brands/official/channel.json` 与实现
  （`tools/zk/src/main.rs:288-300` Brand 结构、`484-500` 校验）都使用 `brand.logo.light/dark`，且 `zk` 不校验
  `powered_by` 枚举（任意字符串都会通过）。
  `docs/README.md:38` 说"版本×渠道、构建、交付"以 `09` 为唯一权威，`docs/README.md:59` 又说品牌 logo 以 `27` 为唯一权威
  → 两份 SSOT 对同一 `channel.json` 字段给出不同 schema。
- **影响**：OEM 客户按 `09` 填 `assets.*` 会构建失败；按 `27` 填 `brand.logo` 则绕过 `09` 的"未知 powered_by 构建红"，
  品牌残留断言（`docs/11:45/66`）失效。
- **修法**：在 `09` §3 用 `brand.logo.light/dark` + `brand.powered_by` 作为唯一字段表（或明确 `27` §4 是唯一 schema 并让 `09` 只链接）；
  `27/23` 只保留视觉/流程规则并引用一处 schema；`zk` 增加 `powered_by ∈ {show,hide}` 与 `assets.app_icon` 的校验；
  `docs/README` §2 指定唯一 owner。

### P0-4 品牌资产/图标未纳入 `24` §3 生成物所有权，Tauri 图标不可复现（C11）
- **证据**：`docs/24-composition-and-runtime-spec.md:224-242` 的 writer 表没有
  `shared/contracts/brand.json`、`apps/desktop/web/public/brand/{logo,logo-dark}.svg`、`apps/desktop/icons/icon.png`；
  但 `tools/zk/src/main.rs:709-735` 实际写 brand.json 与 public 副本（`--check` 也覆盖它们）。
  `apps/desktop/icons/icon.png` 是仓库跟踪文件（1024×1024），`rg` 全仓找不到任何生成器/脚本引用它；
  `docs/27-ui-design-system.md:68-71` 却声明这些"是 `@generated`，禁止手改；`--check` 必须绿"，并说
  "Tauri 应用图标由 `logo.svg` 渲染为 `apps/desktop/icons/icon.png`"。
  `public/brand/*.svg` 也没有 `@generated` 头（复制资产），`docs/27` 的"这些是 @generated"与事实不符。
- **影响**：违反 C11（生成物只能由生成器写、`--check` 必须绿）；图标可被手工替换而与官方 logo 漂移，无法复现/审计。
- **修法**：在 `24` §3 增三行（`brand.json`/public 副本 → `zk compose`；`icon.png` → 生成器或"人工派生 + 校验和+来源"）；
  若要保留"派生"，增加确定性生成步骤（如 `zk compose` 内用 SVG rasterizer 输出 1024 PNG）并纳入 `--check`/CI；
  同步 `27` §4 的表述（复制资产与 `@generated` 文本分开写）。

### P0-5 本地 provider key 明文 JSON 与文档/验收冲突（安全）
- **证据**：`docs/19-client-ux-flows.md:75` "key 只在系统密钥库"、`docs/19:158` "provider key/OAuth token 存系统密钥库"，
  `AGENTS.md:62` "key 存系统密钥库"；实现 `apps/desktop/web/src/i18n.tsx:79`（及 en 版 `:180`）明确写
  "M1 暂存本地 JSON；后续接入系统密钥库"，`screenshots/04-settings.png` 也把这句话展示给用户。
- **影响**：与 AGENTS/19 的验收口径直接矛盾；M1 若按当前实现交付，密钥（provider API key）以明文落盘，
  与"安全责任/密钥管理"承诺不符；C12/C19"文档先于实现"未满足。
- **修法**：二选一并在文档写清：① M1 接入系统密钥库（keyring），`settings.get/set` 不返回明文，UI 提示改为"已保存到系统密钥库"；
  ② 若确要延期，`19/AGENTS` 明确"M1 dev-only 本地 JSON、不得作为 M1 退出条件"，并在 `04/05` 契约与验收里标红；
  同时删除/改写截图中的承诺文案。

## 3. P1（应修：一致性 / 可测试 / 证据）

### P1-1 设计 token 的"唯一 live accent / 阴影 / 线"规则与实现不符
- **证据**：
  - `docs/27-ui-design-system.md:14` "blue-500 只表示 流式中/运行中/已选中/连接中；无 live 状态页面无蓝"；
    实际 `apps/desktop/web/src/app.css:134`（`::selection` 蓝）、`:138`（`focus-visible` 蓝）、`:183`（spinner 蓝）、
    `AssistantComposer.tsx:16`（focus 时 `border-ring` + `ring-2 ring-ring/20`）、`screenshots/13-modular-chat.png`（空闲聚焦 composer 仍是蓝边）。
  - `docs/27:18` "只有 dialog/toast/popover 可 lift；禁止 border+ring+shadow 叠同一边"；实际 `app.css:507`
    窄窗 sidebar 抽屉使用 `--zk-shadow-lift`；`AssistantComposer.tsx:16` 同时叠 border+ring；`sets/index.tsx:72`
    用 inset shadow 表示选中；`ApprovalPart.tsx:41` 审批标题用 `text-destructive`（审批不是错误）。
  - `docs/27:19` "只保留 section 边界 hairline，不做 ledger 式全边框"；markdown 表格仍有逐行 border（`app.css` `.zk-markdown th/td`）、工具卡/输入框也有边框（可解释为控件，但规则需写明）。
- **影响**：设计系统文档无法作为 review/CI 判据；实现与文档互相"合法"。
- **修法**：在 `27` §1 把 non-live 用法清楚列出（焦点环、文本选择、loading spinner、表单控件边/ring 允许；或统一改代码为中性色）；
  把"窄窗抽屉"加入可 lift 浮层清单；明确"控件边框/表格行线"例外；审批标题改中性色或把"审批=destructive"写成例外；
  最好在 `app.css` 把允许的 blue 用途收敛成 `--color-live`/`--color-focus` 两个语义 token。

### P1-2 文档承诺的 drawer Esc / workspace 选择 / 审批卡 a11y 未实现
- **证据**：`docs/27-ui-design-system.md:33` "侧栏抽屉，workspace/session 选择在抽屉顶部；抽屉可用 Esc 关闭"；
  全仓 `apps/desktop/web/src` 与 `packages/ui` 无 `Escape`/keydown 处理（`rg` 无结果）；workspace 选择不存在
  （`docs/26`/C18 仍待做，`AGENTS.md:27`）。`docs/19:110` 要求审批"dialog role、aria-labelledby、焦点陷阱、Esc=拒绝"；
  实现 `packages/ui/ui-chat/web/src/ApprovalPart.tsx:36-41` 是 inline `role="group"`，无 dialog/焦点陷阱/Esc；
  但 `docs/27:30` 又说审批卡"用 data part 占位，后续接 tool-approval 协议"——`19` 与 `27` 对审批的要求不一致。
- **影响**：a11y/键盘验收（`docs/11:148`、`docs/19:176`）无法通过；文档承诺与 M1 状态不一致。
- **修法**：实现 drawer Esc+焦点回退（并加测试），或把 `27:33` 的 Esc/workspace 标为 "TBD（C18/后置）"；
  把 `19 §4` 审批 UX 标为 M2/target（`27` 占位一致），或实现 dialog 版审批卡并在 `27` 去冲突。

### P1-3 六态/真实后端/axe 证据不足，axe.json 不是合法 JSON
- **证据**：`docs/27-ui-design-system.md:82` 要求 "light/dark/narrow、六态、真实 serve 流式 + axe.json"；
  实际 `apps/desktop/web/screenshots/` 有 01-empty/02-streaming/03-tool-card/04-settings/05-light/06-dark/07-narrow/
  10-real-empty/11-real-chat/12-modular-empty/13-modular-chat，缺 error/offline/cancelled/loading 的截图，
  也没有真实 serve 的"流式中"截图（11/13 是完成态）。
  `apps/desktop/web/screenshots/axe.json` 是 4 个 JSON 对象顺序拼接 + 一行 `SERIOUS_OR_CRITICAL=0`，不是合法 JSON
  （无法被 CI/脚本消费），且只有 empty-chat/streaming-chat/completed-chat/settings-dialog 4 个 label，
  没有 dark/narrow/error/offline/cancelled。
- **影响**：`docs/11:148` axe 门禁与 `19:176` 六态验收无法自动化；评审无法引用可复现证据。
- **修法**：把 axe 报告改为合法 JSON（数组或 JSONL，字段 label/theme/viewport/serious/critical/violations），
  覆盖六态 × light/dark × 1280/720，并跑 `serve` 真实后端；补 08-offline/09-error/10-cancelled/14-real-streaming 截图；
  在 `27` §5 写明生成命令与路径。

### P1-4 `docs/19` 个人版首次使用旅程未实现
- **证据**：`docs/19:69-75` 要求"安装→引导页（欢迎+provider 选择+key 输入+测试连接）→Chat"，验收"key 只在系统密钥库"；
  实际 `App` 启动后直接进入主界面（`App.tsx` boot loading/error → AppShell），没有 onboarding 包/slot（`packages/ui` 无 onboarding，
  `registry.ts` 只生成 Sessions/Chat/Settings），i18n 无 onboarding 词条；provider 配置在 SettingsDialog（`ui-settings`），没有"测试连接"。
  `docs/27:40-46` 的包表也没有 onboarding → `19` 与 `27`/实现不一致。
- **影响**：M1 个人版验收（`docs/12` M1 退出条件）与 UX SSOT 不一致；"首装→配 key"无法按文档验收。
- **修法**：二选一：① 实现 onboarding（新增 `@zeokeel/ui-onboarding` 包 + `ui.onboarding` slot + `testConnection` IPC）；
  ② 把 `19 §3.1` 标为 M1.1，M1 以"设置内配置 provider"为验收，并同步 `27 §3` 包表与 `12` 路线图。

### P1-5 手写镜像 DTO（ui-kit types）与"ts-rs 生成 TS"的文档/生成物冲突（C10/C17）
- **证据**：`docs/16-ui-packages.md:219-228`（serde DTO → ts-rs → `shared/ts/*.ts`）、
  `docs/24:237`（`shared/ts/*.ts` 由 `cargo test export_bindings` 写）；实际仓库 `shared/` 只有 `contracts/*.json`，
  没有 `shared/ts/`；`packages/ui/ui-kit/web/src/types.ts:16/26/35/43/91/117/125` 手写 "Mirrors zk_contract_*" 的 DTO，
  无 `git diff` 漂移门禁；`apps/desktop/web/src/protocol.ts` 用 ui-kit guards 做运行时校验（非 Ajv/JSON Schema）。
- **影响**：C10"边界运行时校验"与 C4/契约单实例在 UI 侧没有生成链；Rust DTO 变更时 UI 类型可能静默漂移。
- **修法**：实现 ts-rs export_bindings 输出到 `shared/ts`（或明确 M1 手写镜像为 T6 后置并加契约 fixture 对拍测试）；
  `24` §3 的 writer 与 `16` §5 保持一致；协议边界切到生成 schema + Ajv（C10）。

### P1-6 `ui-kit` 依赖/职责与文档不符，Radix 共享件留在 app shell（C17）
- **证据**：`docs/27:42` 说 `ui-kit` 依赖 "react/Radix"、内容含 Button/Switch/StatusDot/Chip；实际
  `packages/ui/ui-kit/web/package.json` 的 `dependencies: {}`，无 Radix；Radix Tooltip 由 app 直接依赖并使用
  （`apps/desktop/web/package.json` `@radix-ui/react-tooltip`，`App.tsx:1`/`:17`/`:23`）；`docs/27:53` 要求"共享视觉/交互进入 ui-kit"，
  `docs/25:23` 也说 Radix 包装只放 ui-kit。
- **影响**：共享交互（tooltip/焦点/定位）在 shell 内自建，第二份实现风险；文档的依赖表与实际不符。
- **修法**：把 Tooltip 包装移入 `ui-kit`（`primitives.tsx`），app 只消费；或若坚持 shell 自有，更新 `27:42/53` 与 `25` 说明例外并登记
  `@capability`；补 ui-kit 的 Radix 依赖（若移动）。

### P1-7 `docs/16` 的 `zeokeel.client`/baseline/lint-boundaries 是"未来机制"却写成"现状"
- **证据**：`docs/16:4`、`:52-68`、`:70-76` 的清单/baseline/lint 规则为现状语气；`:77-80` 又说 M1 不用。
  实际 `scripts/` 无 `lint-boundaries.mjs`（`ls scripts`）；`package.json` 无 `zeokeel` 清单。
- **影响**：读者会按"必须"实现一个不存在的门禁；C19 评审无法判断当前边界是否成立。
- **修法**：与 P0-1 一并处理：M1 段落只保留实际生效的 `zk.toml` contributions + registry + `@source`；`zeokeel.client`/baseline/lint
  全部标 "target/T21"，并给出启用条件。

### P1-8 审批/错误/离线的语义和证据与 `19` 不一致（含截图暴露）
- **证据**：`docs/19:43` "六态 + 长会话 + 取消/重试"、`19:176` "所有页面六态 + 键盘/axe"；`13-modular-chat.png` 是完成态，
  无 error/offline/cancelled 证据；审批卡（`ApprovalPart.tsx`）在 `19` 中是"安全关键 UI"（diff/风险级别/超时/默认焦点拒绝），
  实现只有 tool/args/reason/decision 四字段；`docs/27:30` 承认占位。
- **影响**：审批是安全关键 UI，M1 以占位进入会与 `19` 验收冲突；`19` 的"必测状态"在 M1 无对应证据。
- **修法**：把 `19 §4` 标为 M2 目标并给 M1 最小审批契约（tool/args/reason/approve/reject/timeout），或扩 `19` 的 `ApprovalView`
  字段并实现 diff/风险/超时；`19` 的六态验收限定到 M1 实际页面。

## 4. P2（建议）

1. `docs/README.md:24` "要改 UI/品牌/颜色 … 评审见 `11` §7"：`11` §7 是"真实模型与外部依赖"，评审在 `11` §9（C19）；应改链接。
2. `docs/23-support-oem.md:125-132` 重复 `27 §4` 的 logo 规则（README §2 已定 27 为唯一权威）；建议 `23` 只留一句"见 27 §4"。
3. `docs/11-testing-and-evidence.md:103` 标题仍为"（自研聊天层）"，与 `27:28`"不自己写聊天层"/AGENTS:64"assistant-ui primitives"不一致；改标题。
4. `apps/desktop/web/README.md:14` 指向已被删除的 `src/assistant/runtime.ts`（实际在 `packages/ui/ui-chat/web/src/runtime.ts`）。
5. `apps/desktop/web/design-notes.md` 是 `27` 的第二份规范（token 表/规则/依赖表重复），且标题编号错误
   （`## 6` 下出现 `### 5.1`/`### 5.2`，`:94-96/:116`）；它与实现也有偏差（`:19` "不在同一边叠 border+ring+shadow" vs composer focus；
   `:120` "阴影只 dialog/toast" vs drawer shadow）。建议改为"实现说明 + 链接 27"，删除规范表。
6. `docs/27:20` "正文 14/400，行高 1.65" vs `app.css:128` body `line-height: 1.6`（markdown 1.65）；二选一。
7. `docs/27:69` 把 public/brand 副本与 brand.ts/brand.json 统称"@generated"；副本无头，建议分开描述（"由 compose 复制，勿手改，--check 校验"）。
8. `brands/official/logo.svg` 与 `logo-dark.svg` 的唯一差异是背景 rect `#FFFFFF` vs `#000000`（`diff` 行 28），
   即"深/浅 logo"只是黑白底板；在纸面/深色侧栏上会形成硬边方块。建议另出透明 mark 变体或说明这是有意资产形态。
9. `apps/desktop/icons/icon.png` 为白底 1024 图标（暗色任务栏会显示白方块）；若保留派生，建议生成透明/暗色适配并纳入 P0-4 的生成器。
10. `docs/27:12` `--tint: 250` 仅作文档/根变量记录，`app.css` 各 token 硬编码 250；改 tint 时不会联动。建议用 `oklch(… var(--tint))` 或删掉误导性变量。
11. `docs/27:51` "app 只从 registry 取组件"：`App.tsx:3-13` 直接从 `@zeokeel/ui-kit` 引入 BrandLogo/Button/Chip/StatusDot 等基础件。
   建议改为"业务 slot 组件只从 registry；基础 kit 组件可直接用"。
12. `packages/ui/ui-kit` 的公共导出无 `@capability` 注释（`docs/25:27/63`，C17/T20 待落地）；`shared/contracts/capability-index.json`
    目前只有 contract.*。建议 ui-kit 共享项登记或在 `25` 明确 UI kit 的登记范围。

## 5. 逐项回答 lead 的问题

### 5.1 文档 vs 实现（颜色/token/字体/圆角/线/阴影/唯一 live accent）
- 颜色/token：**一致**。`app.css:15-40`（light）/`:42-56`（dark）与 `27:12-18` 的 tint 250、bg/fg/muted/border、blue-500、
  destructive、overlay 数值一致；dark 值与 `27:13` 一致；截图 05/06/07/13 的观感与 token 相符。
- 字体：**一致**（`27:20` Public Sans + JetBrains Mono，本地 `@fontsource/*`，`main.tsx:1-5`；无 CDN/衬线；中文回退在 `app.css:35-39`）。
  唯一差异是正文行高 1.6 vs 文档 1.65（P2-6）。
- 圆角：**一致**（`app.css` `@theme` 的 page/document/control/surface/xl/thread/capsule 与 `27:17` 一致，代码用 `rounded-*`）。
- 线/阴影：**部分不一致**（P1-1）：composer focus 叠 border+ring、drawer 用 lift、选中行用 inset shadow、审批用 destructive；
  文档的"只…/禁止…"过绝对。
- 唯一 live accent：**规则不一致**（P1-1）：focus ring/::selection/spinner/switch 都用 blue-500；`13-modular-chat.png` 的空闲聚焦
  composer 即整框蓝（border+ring），与"无 live 状态页面无蓝"冲突。

### 5.2 UI 包边界是否成立？
- **成立的部分**：`App.tsx`/`registry.ts` 是唯一业务组件入口（`App.tsx:12` import registry；`registry.ts:3-5` 只 import 三个业务包）；
  `ui-chat`/`ui-sessions`/`ui-settings` 的 `package.json` 只依赖 `@zeokeel/ui-kit`（+ 各自 Radix/assistant-ui）；
  `ui-kit` 不依赖任何 UI 包；`@assistant-ui/*` 仅出现在 `packages/ui/ui-chat`（`rg` 全仓除 docs/README 外只在 ui-chat）；
  app 内 `components/**`、`assistant/**` 已删除，`settings-validation.ts` 已迁入 `ui-settings`（`design-notes:91`）——未发现第二份实现。
- **不成立/需修**：`27:51` "app 只从 registry 取组件"与 App 直接使用 ui-kit 基础件不符（P2-11）；
  Radix Tooltip 留在 shell 而非 ui-kit（P1-6）；`packages/ui/ui-kit/web/package.json` 的依赖与 `27:42` 不符（P1-6）；
  `ui.sessions` slot 不在 `16` SLOT_DEFS 且 compose 不校验（P0-1）。

### 5.3 logo 规则
- **官方 logo 是否只从渠道配置构建期注入**：是（唯一 UI 渲染组件 `BrandLogo.tsx:1-17`，读取 `UiValue.brand`；
  `brand.ts` 由 `tools/zk/src/main.rs:900-913` 生成，路径来自 `brands/official/channel.json`；`public/brand/*` 由 compose 从
  `brands/official/*` 复制，`sha256` 与源一致）。UI 代码没有硬编码官方素材路径（只有生成的 brand.ts 常量）。
- **仓库有无其它 logo 产物**：`brands/official/{logo,logo-dark}.svg`（源）、`apps/desktop/web/public/brand/*`（compose 复制，跟踪）、
  `apps/desktop/web/dist/brand/*`（构建产物）、`apps/desktop/icons/icon.png`（跟踪的派生图标）。前两类符合 `27 §4`，**icon 不符合**
  （无生成器/未进 24 §3，P0-4）。
- **Tauri icon 是否可视为官方 logo 派生**：视觉上是（1024 PNG 的渐变 logo + 白底），但没有生成脚本/来源/校验和，
  不能算"可复现派生"；且白底在暗色任务栏可能不合适（P2-9）。
- **其它渠道 logo schema 是否够用/安全**：字段上只有 `brand.name/powered_by/logo.light/dark`，`09` 还要求
  `assets.app_icon/accent`，两套 schema 冲突（P0-3）；实现不校验 `powered_by` 枚举、logo 类型/大小/路径（P0-2）→ 不够安全。
- **C15**：未发现把 `zeokeel/channels`、`ZeoKeel-Harness`、`ZeoKeel-Next` 作为依赖/构建输入/索引仓/数据源；
  代码依赖只有 `@assistant-ui/*`/Radix（npm，允许），`27 §6` 只引用其设计文档。**未违反 C15**。

### 5.4 a11y/六态/窄窗/深色/中文/键盘/axe 证据
- 中文：默认 zh-CN，en-US 词典完整；`controller.ts:165` 同步 `document.documentElement.lang`；截图为中文，**基本够**。
- 键盘：composer Enter/Shift+Enter/IME 有 Vitest（`tests/composer-keyboard.test.tsx`）；skip link、focus-visible、tooltip、dialog 有；
  **drawer Esc 缺失**（P1-2）；审批卡默认焦点/Esc 拒绝未实现（P1-2）。
- 窄窗：`07-narrow.png` 只展示抽屉关闭态；无抽屉打开/关闭（Esc/backdrop/focus）证据（P1-2/P1-3）。
- 深色：`06-dark.png` + token 对照一致；但 axe 未覆盖 dark（P1-3）。
- 六态：Vitest `app-states.test.tsx` 覆盖 loading/empty/error/offline，`tests/chat-flow` 覆盖 streaming/cancel；但截图/axe 缺
  error/offline/cancelled/loading 的完整证据；`19:176` "所有页面六态" 与 M1 实际页面范围也不一致（P1-3/P1-8）。
- axe：`apps/desktop/web/screenshots/axe.json` 不是合法 JSON（见 P1-3），4 个 label、0 serious；不足以支撑 `11 §8.4` 门禁。
- `13-modular-chat.png` 暴露的问题：空闲 composer 聚焦即整框蓝（border+ring），违反 `27:14/:18` 的字面规则；
  真实后端截图是完成态，未证明"真实 serve 流式"；页面无 MOCK badge（正确），但无 axe 对应证据。

### 5.5 文档过时/矛盾（本问题清单）
- **AGENTS 3.1 与 27 是否冲突**：现在**不冲突**——`AGENTS.md:64` 已改为 "assistant-ui primitives + Radix + Tailwind 4；
  流式 Markdown `@assistant-ui/react-markdown`"，与 `27 §2` 一致；但 `docs/11:103` 仍写"自研聊天层"，需改（P2-3）。
- **docs/16 的 `zeokeel.client` vs M1 `[[ui.contributions]]`**：**矛盾已部分修复但同一文件内自相矛盾**：
  `16:77-80` 已声明 M1 不用 `zeokeel.client`，但 `16:4`/`16:52-68` 仍说"必须带清单"。（P0-1/P1-7）
- 其它：`09` vs `27/23` 渠道 schema（P0-3）；`27` vs `16` registry 文件名/virtual（P0-1）；
  `19` vs `27` 审批占位（P1-2/P1-8）；`19` onboarding vs `27`/实现（P1-4）；`README:24` 评审链接（P2-1）；
  `23` 重复 `27` logo 规则（P2-2）；`24 §3` 缺品牌/图标 writer（P0-4）。

## 6. 复评条件（建议按顺序）

1. 修 P0-1~P0-5（其中 P0-2/P0-3/P0-4 涉及 `tools/zk` 与 `docs/24/27/23/09`，P0-5 涉及 `ui-settings/i18n` 与 `docs/19/AGENTS`）。
2. 修 P1-1~P1-8（至少 P1-1/P1-2/P1-3/P1-4 要在本次 UI 方案批准前完成）。
3. 产出合法 axe JSON + 六态/light/dark/narrow/真实 serve 证据文件，并在 `27 §5` 写命令。
4. 更新 `docs/11` §8 标题、`README` §1 链接、`23` 去重、`design-notes` 去规范化；`docs-check`/`spec-check` 仍 0/0。
5. 同一评审者复评（本文件"复评结果"追加）；APPROVED 后才按 C16 建 spec/施工（本方案当前**无关联 spec**）。

## 7. 复评结果

- 首评：2026-10-30，`CHANGES-REQUESTED`（P0=5，P1=8，P2=12）。
- 复评：待实现者修复 P0/P1 后追加；未复评前 `docs/27` 不得作为施工依据（C19/`11` §9.3）。

## 8. 参考证据命令（可复现）

```bash
# 文档/实现对照
rg -n "zeokeel\.client|ui-shell-kit|SLOT_DEFS|registry\.gen\.ts|shared/ts" docs/16-ui-packages.md
rg -n "brand\.logo|assets\.logo_primary|powered_by" docs/09-delivery-and-oem.md docs/27-ui-design-system.md docs/23-support-oem.md
rg -n "brand\.logo|powered_by|canonicalize|join\(" tools/zk/src/main.rs
rg -n "blue-500|ring-ring|shadow-\[|text-destructive" apps/desktop/web/src packages/ui --include=*.css --include=*.tsx
rg -n "Escape|keydown" apps/desktop/web/src packages/ui
rg -n "@assistant-ui" packages/ui apps/desktop/web/package.json
ls shared; find . -iname '*logo*' -not -path './node_modules/*' -not -path './target/*'
git ls-files apps/desktop/icons apps/desktop/web/public/brand
python3 -c "import json;json.load(open('apps/desktop/web/screenshots/axe.json'))"   # 当前会失败（拼接对象）

# 门禁（本次）
python3 scripts/docs-check.py
python3 scripts/spec-check.py
```

---

# 复评（2026-10-30）

## R0. 复评元信息（按 `docs/reviews/README.md` 必填字段）

- 评审对象：工作树（HEAD `f48b87f` + 未提交修订）的 UI/品牌/工作区文档集。快照 SHA256：
  - `docs/27-ui-design-system.md` `892b38e3981cd5cca7007f314ad3fd26776b53a604b776169730c5c00ed2c9ad`
  - `docs/16-ui-packages.md` `1903106994425d437c0d8e3325ed77dc6da5588f59d0a90c36a5ce64fa3894aa`
  - `docs/19-client-ux-flows.md` `39346fd896a091a417e0aa2d1c09947b309b227ee375d0ca1de6ed678dc10403`
  - `docs/09-delivery-and-oem.md` `a89f7c01b8f1ea5f9b92291767c5d3b7a26d3db64ac1812d1075e44e48f787b6`
  - `docs/23-support-oem.md` `2e3c7614ce74711c708fb0c1f9e0bd78f3d4aabd3fb1aa7e83f6a4c7b4f12441`
  - `docs/24-composition-and-runtime-spec.md` `bdcecc80651b2f6a7f2d64c01f33a28e4aad59d3591900e6e0925d5ae6394f5b`
  - `docs/26-workspace-and-conversations.md` `872d64b742c23b7c419ee1fcb02fcb9aa621563389e71a6f8ed0a8c754a60b6a`
  - `AGENTS.md` `147a538a14d3e5dce416318ff45b5c0747062166876a8d2f62e78076ed085d84`
  - `docs/11-testing-and-evidence.md` `14b40b03c042ce4e0a4a73444b569dc9cbbf9f2b3a555c058d7112ac457bf9b1`
  - `docs/README.md` `3bc66f7104bd1fb2429115fe00d468570e47041cd2f974d1878f8d3839f5bb08`
- 评审者：`audit-runtime`（非实现者；只读评审，未修改规范/代码；独立于实现者）。
- 复评日期：2026-10-30。
- 结论：**CHANGES-REQUESTED**（P0=1，P1=4，P2=8）。
- 关联 spec：尚未创建；建议 `specs/changes/2026-10-30-ui-design-system/`（C16 五件套，proposal 引用本记录，acceptance 回链）。
- 失效条件：上述任一文档在本次复评后变更（hash 变化）→ 本记录失效，需重新评审（`docs/reviews/README.md`）。

## R1. 上一轮 P0/P1 逐条状态

| 上轮项 | 状态 | 证据 |
| --- | --- | --- |
| P0-1 docs/16 vs M1 实现 | **RESOLVED（文档层）** | `16` 重写为 M1 现状（`16:3-14`、`16:55-63`、`16:71/78`）并把 zeokeel.client/SLOT_DEFS/registry.gen/virtual/lint-boundaries/ts-rs 全部移入 §6 target（`16:93-103`）；不再与 `27 §3` 冲突。 |
| P0-2 logo 路径安全 | **RESOLVED（文档层）** | `27:78-80` 与 `09:118-119` 写明 canonicalize/渠道内相对路径/禁 `..` 绝对路径 symlink/256KB/拒绝 script 外链；`24:239` writer 校验路径/大小/内容。代码校验属施工项（R3）。 |
| P0-3 09 vs 27/23 渠道 schema | **RESOLVED（文档层）** | `09:118-122` 已统一为 `brand.logo.{light,dark}` + `brand.icon_source` + `brand.powered_by` 必填枚举；`23:125-129` 删重复、只链接 `09`/`27`。 |
| P0-4 品牌资产/图标 writer | **PARTIAL** | `24:239-240` 已补 brand.json/public logo/brand.ts 与 icon.png 行；但 icon 的 writer `scripts/gen-icons.sh` **在仓库中不存在**（`ls scripts`/`find` 无该文件），`09:118-119`/`24:240`/`27:82` 以现状语气引用它 → 见 P1-3。 |
| P0-5 key 明文 JSON vs 文档 | **PARTIAL** | 已修 `AGENTS:62`（M1 JSON 0600/M2 keyring）与 `27:26-29`（M1 例外）；但 `19:76/159` 仍写"key 只在系统密钥库/存系统密钥库" → 见 P1-1。 |
| P1-1 token 规则 vs 实现 | **RESOLVED（文档层）** | `27:14-18` 已把焦点环改 foreground/muted、允许 drawer lift、composer 聚焦只改边框；代码修复属施工项（R3-3）。 |
| P1-2 drawer Esc/workspace/审批 a11y | **RESOLVED（文档层，范围）** | `27:39` 抽屉 Esc；`26 §6` 定义 Workspace switcher/首用/Esc/六态；`19:33-34` 标注 Workspace 前置与 M1 范围（onboarding/审批 post-M1）。代码实现属施工项（R3-5），且需等 `26` 的另一份评审（当前 CHANGES-REQUESTED）通过。 |
| P1-3 axe/六态证据 | **RESOLVED（文档层，要求保留）** | `27:92` 与 `16:89-91` 仍要求合法 axe JSON/六态/真实 serve；证据属施工项（R3-6）。 |
| P1-4 19 onboarding 未实现 | **RESOLVED（文档层）** | `19:34` 明确 onboarding post-M1；M1 由设置弹窗覆盖 provider。验收清单仍矛盾 → P1-2。 |
| P1-5 手写 DTO vs ts-rs/Ajv | **REMAINING（升级为 P0-1）** | `16:78` 把 ts-rs/Ajv 标 T21 目标，`16:101` 又把"ts-rs/Ajv（C10）"放进 post-M1 目标列表；与 `AGENTS:45` C10"外部输入必须 Rust 生成 JSON Schema + Ajv"冲突。 |
| P1-6 Radix/ui-kit 依赖 | **RESOLVED（文档层）** | `27:48` 标 ui-kit M1 依赖 react；`27:50/58/59` 写明 Radix Tooltip M1 在 shell、T21 迁入；`25` 的共享规则有例外说明。代码迁移属 T21。 |
| P1-7 16 的 zeokeel.client 现状语气 | **RESOLVED** | `16:13-14` 明确"M1 不使用"；§6 单列为 target。 |
| P1-8 审批/六态与 M1 范围 | **RESOLVED（文档层）** | `19:34` 审批占位/post-M1；`27:36` 审批 data part 占位；验收清单矛盾见 P1-2。 |

## R2. 剩余 P0（阻塞 approved spec）

### 复评 P0-1｜C10 被列为 post-M1/T21 目标，与最高约束冲突（文档/决策）
- 证据：`AGENTS.md:45` C10 "外部输入必须运行时校验：Rust 生成 JSON Schema，前端 Ajv 校验；禁止类型断言代替校验"；
  `docs/16-ui-packages.md:78` "M1 的 DTO 是 ui-kit/types.ts 手写镜像 + guards；ts-rs/Ajv 生成是 T21 目标"；
  `docs/16-ui-packages.md:101` 把 "ts-rs 生成 shared/ts、schemars JSON Schema + 前端 Ajv（C10）" 列入"§6 目标模型（post-M1/T21；尚未实现）"。
- 影响：若按此写 approved spec，M1 会以手写镜像 + 自定义 guard 替代 C10 的生成 schema/Ajv，违反最高约束；C19 评审不能通过。
- 修法（二选一，需用户/Lead 拍板）：
  1. **纳入 M1**：把 ts-rs `export_bindings` + `shared/schema` + Ajv 校验列入 M1 spec 首步，`16 §3/§6` 改为"M1 实现 C10，T21 只做严格闭包/边界增强"；或
  2. **显式例外**：写 ADR（`docs/13`）/D（`docs/15` §C）说明 M1 允许手写镜像 + guard 的期限、补偿措施与退出条件，并经用户确认；同时 `AGENTS` C10 引用该例外。仅把 C10 标 T21 不足以豁免。

## R3. 剩余 P1（应修；先文档，后施工）

### 复评 P1-1｜docs/19 密钥口径与 AGENTS/27 冲突（文档）
- 证据：`docs/19-client-ux-flows.md:76` "key 只在系统密钥库"、`:159` "provider key/OAuth token 存系统密钥库"；
  `AGENTS.md:62` "M1 key 暂存本地 JSON（0600），M2 接系统密钥库"；`docs/27-ui-design-system.md:26-29` 已写 M1 例外。
- 影响：`19` 是客户端 UX/验收 SSOT；与 AGENTS/27 冲突会让 M1 验收/安全口径不可判定。
- 修法：`19:76/159` 改为"按 `27` §1.5：M1 本地 JSON（0600），M2 keyring"；验收文案同步。

### 复评 P1-2｜docs/19 M1 验收清单与 M1 范围自相矛盾（文档）
- 证据：`docs/19:34` "onboarding 与审批完整流程为 post-M1；审批是占位卡"；但 `19:174` M1/M2 退出清单仍写
  "个人版：首装→配 key→首轮对话→工具→审批→…全流程 e2e"，`19:177` "所有页面六态 + …审批卡读屏与超时通知"。
- 影响：M1 退出条件把 post-M1 的 onboarding/完整审批当 M1 验收；无法据以写 M1 spec/验收。
- 修法：把 `19:174` 拆成 M1（设置内配 provider、首轮对话/工具/占位审批/重启恢复）与 M2（onboarding/完整审批）两行；
  `19:177` 的审批卡读屏/超时移到 M2，或标"post-M1"。

### 复评 P1-3｜docs/19 槽位引用失效（文档）
- 证据：`docs/19-client-ux-flows.md:60` "slotId 由 `16` §3 的 `SLOT_DEFS` 冻结…第二列是 slotId（`16` §3）"；
  但 `16` 已无 §3/SLOT_DEFS：M1 实际仅 3 个 slot（`16:55-63`：ui.sessions/ui.chat/ui.settings），完整 SLOT_DEFS 在 `16:103` §6 列为 T21 目标。
  而 `19:38-58` 仍列 `ui.shell`/`ui.composer`/`ui.toolcall`/`ui.approval`/`ui.onboarding`/`ui.login`/`ui.first-run`/`ui.gate.*` 等 M1 不存在的 slot。
- 影响：19 的"页面/slotId 冻结"无法与 16/27/实现对应；读者会按不存在的 SLOT_DEFS 实现。
- 修法：`19:60` 改为"引用 `16` §2.2 M1 slot + `16` §6 T21 SLOT_DEFS 目标"；在表头注明"M1 仅 ui.sessions/ui.chat/ui.settings 受 16 §2.2 约束，其余为 post-M1/T21"；或补 `ui.shell` 到 16 §2.2（不推荐，和实现不符）。

### 复评 P1-4｜`scripts/gen-icons.sh` 被当作既有 writer，但文件不存在（文档/施工）
- 证据：`docs/09-delivery-and-oem.md:118-119`、`docs/24-composition-and-runtime-spec.md:240`、
  `docs/27-ui-design-system.md:82` 都以现状语气引用 `scripts/gen-icons.sh` 生成图标；`ls scripts/` 与 `find . -name 'gen-icons*'` 均无该文件。
- 影响：`24` §3 的 writer 表对 `icons/icon.png` 不可执行；C11 要求的"--check 绿"无法实现；实现者不知道脚本的输入/输出/尺寸/失败条件。
- 修法：二选一：① 在 `27/09/24` 标"（脚本待新增，随本方案 M1 spec 施工提交）"，并在施工清单补脚本契约
  （输入渠道 `logo.svg`/`icon_source`，输出 `apps/desktop/icons/icon.png`，1024 RGBA，确定性，`git diff --exit-code`；是否含 ico/多尺寸另定）；
  ② 若坚持现状语气，则本复评通过前必须把脚本加入仓库**并**纳入 CI/`--check`。建议①（与"代码修复后置"一致）。

## R4. 剩余 P2（建议，不阻塞 spec）

- **P2-1** `27:57` 规则 2 "app 只从 registry 取组件" 与 `27:58` "M1 的 app 可直接使用 ui-kit 基础件" 字面矛盾；
  改规则 2 为"业务 slot 组件只从 registry；基础 kit 组件可直接用"。
- **P2-2** `16:3` 状态"M1 已实现（2026-10-30）"与 `16:89-91` 的 Playwright/axe 证据（尚未产出）口径不符；
  建议改"M1 包模型已实现；视觉/证据待 §5 完成"（C19/证据政策）。
- **P2-3** `19:38-58` 页面表未含 Workspace 首用/switcher 页面（`26 §6` 已有），且 `19:33` 已声明 Workspace 前置；
  建议加"Workspace 选择/首用"行（slot 归 `ui-sessions`）或在表头注明由 `26 §6` 冻结。
- **P2-4** `27:49` ui-sessions 内容未列 Workspace switcher，但 `26 §6`/`16:59` 有；建议 27 包表补"Workspace switcher"。
- **P2-5** `27:70` 的 channel.json 示例只含 brand；`09:118-122` 还要求 `brand.icon_source`/`assets.accent` 等，建议示例标注"节选，完整见 09 §3"。
- **P2-6** `AGENTS.md:27` "尚未完成：真实 provider 的 UI 配置"与 `ui-settings` 已提供 provider 配置 UI 不符（M1 实际是"key 落盘方式待 keyring"）；
  建议改为"provider UI 已有、keyring 未接"。
- **P2-7** `docs/26` 仍是"待 C19 复评"且另一评审记录 `audit-arch` 为 CHANGES-REQUESTED；`16/19/27` 已引用其 Workspace 规则。
  建议在 UI spec 依赖/施工顺序中写"Workspace UI 部分以 `26` 复评通过为准"，不要在本 UI spec 里抢先冻结 workspace 交互。
- **P2-8** `24:240` 的 icon writer 标"T21 接入 CI"但图标是 M1 交付物；M1 若不带 `--check`，图标仍可能漂移。建议 M1 先做"脚本 + 手工运行 + git diff 校验"，CI 化可 T21。

## R5. 文档是否可据以写 approved spec？

- **当前：不可以**。P0-1（C10 冲突）与 P1-1/P1-2/P1-3/P1-4（均为文档编辑/小决策）未闭合；
  按 `docs/11` §9.1，P0/P1 修复并由同一评审者复评后才能写 approved spec。
- 预计修复为**纯文档编辑 + 一个 C10 决策**（约 4 处：`16:78/101` C10 归属、`19:76/159/174/177`、`19:60`、`gen-icons` 措辞），
  无需先改代码；完成后可直接复评（本记录 hash 会失效，需重新记录）。
- 施工阶段才做的代码/证据（与文档正确性无关，但 APPROVED 后必须在 spec/acceptance 中列为任务）：
  1. `tools/zk` brand 校验：channel id 正则、logo canonicalize/渠道内/禁 `..`/绝对/symlink、仅 `.svg`、≤256KB、
     拒绝 `<script>`/外链、`powered_by ∈ {show,hide}`、`icon_source` 路径安全；负向 fixtures。
  2. `scripts/gen-icons.sh`（或等价 `zk assets`）：确定性生成 `apps/desktop/icons/icon.png`（1024 RGBA），
     输入 `brand.logo.*`/`icon_source`；接入 `--check`/CI（T21 可延）；`24` §3 writer 与 `check.sh` 对应。
  3. CSS/UI 修复（按 `27` §1）：焦点环改 foreground/muted、composer 聚焦只改边框（去掉 blue ring）、
     drawer 保持 lift、审批标题去 destructive（或文档列为 warning 例外）、确认 `::selection`/spinner/switch 的 blue 语义并在 27 写清。
  4. key 落盘：M1 settings 文件 0600 + UI 明示本地 JSON；删除/改写"系统密钥库"文案；M2 keyring 任务。
  5. Workspace/抽屉：Esc 关闭 + 焦点回退；workspace switcher/首用/切换取消 turn（依赖 `26` 复评通过）；Vitest/Playwright。
  6. 证据：合法 axe JSON（数组/JSONL；label/theme/viewport/serious/critical/violations），覆盖六态 × light/dark × 1280/720、
     mock + 真实 serve；补 error/offline/cancelled/loading 与真实流式截图；`19 §10`/`26 §7` 证据路径。
  7. C10：若走纳入 M1，则 ts-rs `export_bindings` → `shared/ts` + schemars schema + 前端 Ajv，替换手写 mirror/guard；
     若走例外，则 ADR/D + 用户确认，并记录退出条件。
  8. `review-check.py` 关联：本记录 APPROVED 后，`specs/changes/2026-10-30-ui-design-system/proposal.md` 必须写 `评审：docs/reviews/2026-10-30-ui-design-system.md`，
     `acceptance.md` 回链本记录与 P0/P1 关闭证据。

## R6. 复评结论

- **CHANGES-REQUESTED**（P0=1，P1=4，P2=8）。
- 结论不变更前，`docs/27/16` 不得作为施工依据（C19/`11` §9.3）；代码修复继续后置。
- 修复 P0-1、P1-1~P1-4 后由同一评审者（`audit-runtime`）复评；复评时请提供本文档集的**新 SHA256** 与逐条关闭证据。

---

# 复评（2026-10-30·第三轮）

## R7. 复评元信息

- 评审对象：工作树（HEAD `f48b87f` + 未提交修订）的 UI/品牌/工作区文档集。当前 SHA256：
  - `docs/27-ui-design-system.md` `9d83da4f71f9ef5ea0a3f5b3a4ac52d1a44c9677a57bc791ccbdb158da82b334`
  - `docs/16-ui-packages.md` `3f0421614e9ca0a4ec56dc244986d189566cd762bb7f591cd9f038cefc5f68f2`
  - `docs/19-client-ux-flows.md` `58d926cf7e01abc59a3ea47b15ea79790d50c23ecadc0baa367c172888f22b01`
  - `docs/09-delivery-and-oem.md` `a89f7c01b8f1ea5f9b92291767c5d3b7a26d3db64ac1812d1075e44e48f787b6`
  - `docs/23-support-oem.md` `2e3c7614ce74711c708fb0c1f9e0bd78f3d4aabd3fb1aa7e83f6a4c7b4f12441`
  - `docs/24-composition-and-runtime-spec.md` `cbe4bc073b17a48cbe64537fd165d4807230b02f414dcf7da31ffeae94239769`
  - `docs/26-workspace-and-conversations.md` `99745c531eb9156a89e121ceb2973a2d5a653d0d0850e668c47c20c43a1bcf34`
  - `AGENTS.md` `b6ab9ffc9c7c24a7cba68a5df18bc6cf234f2e8da99e39d2600ef792e9daf7b8`
  - `docs/13-risks-and-adr.md` `55a3e8870ff03ba3ff2bef2fb876586e729c85d01ce4c642cfb28e25e56fe20d`
  - `docs/15-open-questions.md` `60cc5b0d0ad4793c18e8ad0a05e78ea7c82c17cafa8ebad3b3d43054ed825a9d`
  - `docs/11-testing-and-evidence.md` `f47a8ae2b6461d2dbfc3f022d39148f8e78cb37b2f45a8bcddf22cb44efb9a64`
  - `docs/README.md` `28a693db26d853848990f8e3a6f8c8a195e13b3e0eecc1b6f571aad4eecddc17`
- 评审者：`audit-runtime`（非实现者；只读评审，未修改规范/代码；独立于实现者）。
- 复评日期：2026-10-30。
- **结论：APPROVED（文档层；P0=0，P1=0，P2=8）**。
- 关联 spec：建议 `specs/changes/2026-10-30-ui-design-system/`；proposal 必写 `评审：docs/reviews/2026-10-30-ui-design-system.md`，acceptance 回链本记录与施工清单。
- 失效条件：上述任一文档变更（SHA256 变化）→ 本记录失效，需重新评审（`docs/reviews/README.md`）。

## R8. 上一轮 P0/P1 关闭确认

| 项 | 状态 | 证据 |
| --- | --- | --- |
| P0-1 C10 例外 | **RESOLVED** | `13:500-513` ADR-059 `accepted`（时间盒/4 条约束/硬退出 T22）；`15:159` D67；`AGENTS:45` C10 引用 ADR-059/T22；`16:78` M1 手写镜像明确标 ADR-059/T22。 |
| P1-1 19 密钥 | **RESOLVED** | `19:77`、`19:160` 改为 M1 本地 JSON(0600)/M2 keyring，与 AGENTS:62、27:26-29 一致。 |
| P1-2 19 验收 | **RESOLVED** | `19:34` M1 范围；`19:72/74` onboarding/审批 post-M1；`19:176/178` M1 验收已拆分，审批/回读屏移 M2。 |
| P1-3 19 slot | **RESOLVED** | `19:61` 改为 M1 仅 ui.sessions/ui.chat/ui.settings（引 `16 §2.2`），其余 post-M1/T21；补工作区切换行（`19:44`），`ui.shell` 行标 M1 无（`19:43`）。 |
| P1-4 gen-icons | **RESOLVED** | `scripts/gen-icons.sh` 已存在（可执行；channel logo → 1024 RGBA；默认 official；`rsvg-convert` 缺失时报错），09:118-119/24:240/27:82 的引用有对应实体；`24:240` 保留 writer/`git diff` 校验。 |
| P2-1 27 规则 2/3 | RESOLVED | `27:57` 已限定"业务 slot 只从 registry；基础件可直接用 ui-kit"。 |
| P2-2 16 状态/证据 | RESOLVED | `16:3` "M1 已实现；证据补跑见 §5"；`16:89-91` 保留 Playwright/axe/六态要求。 |
| P2-6 AGENTS provider | RESOLVED | `AGENTS:27` 改为"provider 运行时按设置选择/系统密钥库（M1b）"，与 ui-settings 现状一致。 |

**结论：P0/P1 全部关闭，文档可据以写 approved spec。** 以下 P2 与施工项不阻塞 spec（C19 只要求 P0/P1 清零），但建议在 spec 首步/施工中一并处理。

## R9. 剩余 P2（文档，不阻塞）

1. **T22 未进 `15 §C` 索引**：`13:500-513` ADR-059 与 `15:159` D67 都引用 T22，但 `15 §C` 表只到 T21（`15:145-146`）；`AGENTS:302`、`docs/README:107`、`13:562`、`PLAN:149-151` 仍写 T1–T21。
   修法：在 `15 §C` 增 T22 行（"C10 过渡退出：ts-rs/schemars/Ajv → M2 spec 前完成"），并把 AGENTS/README/13/PLAN 的范围改 T1–T22。
2. **ADR 范围陈旧**：`AGENTS:273` 写 ADR-001…058、`13:552` 写 D1–D66/ADR-001…058、`PLAN:149` 写 D1–D65/ADR-001…057，但已有 ADR-059/D67。修法：统一为 ADR-001…059、D1–D67（15 标题已正确）。
3. **27 `ui-sessions` 用途未写 Workspace switcher**：`27:49` 仍只有"会话列表/新建/选中/设置入口"；`16:59` 与 `26 §6`（侧栏顶部=Workspace switcher）要求包含。修法：27 包表补 "Workspace switcher（切换/新建/重命名/移除）"。
4. **`icon_source` 与 gen-icons 不一致**：`09:119` 定义可选 `brand.icon_source`（缺省由 logo.light 派生），但 `scripts/gen-icons.sh:8` 固定读 `brands/<channel>/logo.svg`，不读 `channel.json`/`icon_source`；`24:240`/`27:82` 也只说 logo。M1 official 不受影响，但 OEM/M5 行为与文档不符。
   修法：脚本读取 `channel.json` 的 `brand.logo.light`（可选 `icon_source`），或文档把 `icon_source` 标"T21 扩展、当前脚本仅 logo.light"；`24/27` 同步。
5. **gen-icons 依赖未文档化**：`scripts/gen-icons.sh:13` 需要 `rsvg-convert`（librsvg2-bin），但 `10/18/27` 未把它列入系统依赖/CI 镜像；确定性依赖 rsvg 版本（脚本注释已写）。修法：`10 §8`/CI 镜像/`27 §5` 写明；或改为仓库内可控 rasterizer。
6. **27 的 C10 指针缺失（可选）**：`27 §3` ui-kit "DTO/类型"未提 `16 §3` 的 ADR-059/T22 过渡；建议加一句"DTO 生成/校验见 `16` §3/ADR-059"，避免读者把 `ui-kit/types.ts` 当最终方案。
7. **27 channel.json 示例是节选但未标注**：`27:68-71` 只有 brand；`09 §3` 还有 `icon_source`/`assets.accent`/`defaults` 等。修法：加"（节选，完整见 `09 §3`）"。
8. **26 仍未通过 C19**：`26:3` "M1 冻结候选（待 C19 复评）"，`docs/reviews/2026-10-30-workspace-and-process.md` 仍 CHANGES-REQUESTED；`16/19/27` 已引用 Workspace 交互。
   修法：UI spec 可将"Workspace switcher/首用/切换"依赖 `26` 复评通过；施工顺序以 `26` 复评为准，不把 workspace UI 当已完成前置。

另：`24:240` 仍标 icon "T21 接入 CI"，而 icon 是 M1 交付物；M1 至少应"脚本 + 手工运行 + `git diff`"作为门禁，CI 化可 T21（P2，施工项）。

## R10. 施工阶段必须完成的代码/证据清单（APPROVED 后写入 spec/acceptance）

1. **品牌/logo 生成安全**（`tools/zk`）：
   - channel id 正则；`brand.logo.{light,dark}` canonicalize 后必须位于渠道目录内，拒绝 `..`/绝对路径/symlink 逃逸；仅 `.svg`、≤256KB；拒绝 `<script>`/外链；`brand.powered_by ∈ {show,hide}`；`brand.icon_source` 路径安全；
   - `brand.json`/`public/brand/*` 由 compose 生成并被 `--check` 覆盖；负向渠道 fixtures + 单测。
2. **icon 派生**：`scripts/gen-icons.sh` 读取 `channel.json`（logo.light，可选 icon_source）→ 1024 RGBA；确定性（同输入/同 rsvg 版本两次 sha256 一致）；`git diff --exit-code`；CI 化（T21）；`rsvg-convert` 依赖写入 CI 镜像/文档。
3. **CSS/视觉按 `27 §1` 收口**：焦点环 foreground/muted；composer 聚焦只改边框色、去掉 blue ring；`::selection`/spinner/switch/live 的 blue 语义在 27/实现一致；审批标题去 destructive（或文档列为 warning）；drawer lift 保留；`prefers-reduced-motion` 回归。
4. **密钥落盘**：settings 文件 0600；UI 文案"本地 JSON（0600）"；日志/崩溃/导出不含 key；M2 keyring 任务。
5. **Workspace/窄窗**：drawer Esc 关闭+焦点回退；workspace switcher/首用/切换取消 turn；`26` 复评通过后按 `26 §9` 实现；Vitest/Playwright。
6. **证据**：合法 axe JSON（数组/JSONL；label/theme/viewport/serious/critical/violations），覆盖六态 × light/dark × 1280/720 × mock+真实 serve；补 error/offline/cancelled/loading 与真实流式截图；`19 §10`/`26 §7` 证据路径；Playwright 真实操作。
7. **C10 过渡追踪**：ADR-059/D67/T22 进入 M2 spec；M1 保持 guard 负向测试；T22 未完成 CI warning/跟踪 issue，完成前不得发布 M2。
8. **spec 回链**：proposal 引用本记录；acceptance 填 P0/P1 关闭证据 + 上述 1–7 的验证命令/证据路径。

## R11. 最终结论

- **APPROVED（文档层）**：P0=0，P1=0，P2=8（见 §R9）。
- 文档可据以写 `specs/changes/2026-10-30-ui-design-system/` 的 approved spec；本 APPROVED **不代表代码已实现/证据已齐**，施工清单见 §R10。
- 若任一被评审文档在最终 spec 前变更（SHA256 变化），本记录失效，需重新评审；建议在 spec proposal 中记录本记录最新 SHA256。
- 本记录历史段落（首评/前两轮）保留；本节的 APPROVED 是当前最新结论。
