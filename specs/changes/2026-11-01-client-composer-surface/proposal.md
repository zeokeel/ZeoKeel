# 2026-11-01-client-composer-surface · 提案

- 状态：`approved`
- Owner：dsh-ref
- 日期：2026-11-01
- 关联：用户已批准的 Phase 2「运行时能力可见面 + composer 体验」（A 方案全量分期中的 Phase 2a）；`research/client-dsh-parity-2026-11-01.md` §3（附件/`/`/`@`/模型选择/上下文）、§4（P1-5、P2-5）；`target/audits/zk-client-inventory.md` §3（B7）、§5（C7）；`docs/19` §2（输入槽位）、`docs/28` §4（Conversation + Composer）；已实现前置：`2026-11-01-client-settings-ia`（settings revision）、`2026-11-01-client-approval-tool-rendering`（工具行/usage 事件）
- 评审：docs/reviews/2026-11-01-client-phase2-specs.md
- 变更预算：**16 个文件**（Lead 批准例外：含 docs/26 §5 IPC 登记）（契约 1、desktop 1、ui-kit 2、ui-chat 3、desktop-web 4、e2e 1、文档 3；不含本 spec 目录与生成物）；超出模板 10 文件阈值，但为单一目标（composer 输入面）且用户已批 Phase 2 分期，预算口径见 §8

## 1. 背景 / 问题

用户已批准 Phase 2：把「运行时能力可见面 + composer 体验」补齐。当前 composer 是**纯文本 textarea + 发送/停止/字数/离线提示**
（`packages/ui/ui-chat/web/src/AssistantComposer.tsx:9-51`），而 `docs/28` §4 把 `/` 命令、`@` 文件引用、附件、模型下拉、上下文占用
列为 composer 目标（`docs/28-ui-module-map-and-skeleton.md:62` 附近），`docs/19` §2 也把「输入」定义为独立能力行
（`ui.composer`，必测状态含「附件中」）：

- **没有 `/` 命令菜单**：用户可执行命令只存在于 ⌘K 全局面板（`apps/desktop/web/src/App.tsx:591-607` 的 4 条个人版命令），
  在输入框里打 `/` 只是普通字符；DSH 的 composer 内联 `/` 菜单（`target/audits/dsh-reference-inventory.md` §4.4）完全缺失。
- **没有 `@` 文件引用**：右栏文件列表只读（`workspace.files`，`apps/desktop/src/ipc.rs:914`），composer 无法引用文件。
- **没有附件**：无拖放/选择/上传/预览/失败态；`docs/19` §2 的「composing / 附件中 / 超长 / 发送失败」无对应实现。
- **模型与用量不可见**：模型只在设置「模型/本地 Provider」pane 可改；composer 不显示当前模型；
  已实现的 `AgentUsage`/`usage` 事件只在消息尾部聚合展示，输入面没有上下文/用量指示。
- **不做的后果**：Phase 2 的「能力可见面」在最高频入口（输入框）继续断裂；用户无法在对话中快速执行命令、引用文件、
  附加材料，也无法确认当前模型和本轮消耗。

## 2. 目标

- **G1 `/` 命令菜单**：在 composer 输入 `/` 打开命令候选；候选来自**与 ⌘K 同一份命令目录**（不建第二份），
  首期只含既有 client 命令（新建会话/设置/侧边栏/右栏/取消/重试；enterprise 构建额外含技能/连接器/应用面板命令），
  支持 ↑↓/Enter/Esc/过滤，执行后清空触发文本；不引入 Host 命令（`/compact`/`/export` 等）或技能 `/skill`。
- **G2 `@` 文件引用**：输入 `@` 或点击引用按钮打开 workspace 文件候选（复用 `workspace.files`，不新增搜索 IPC），
  过滤后以 chip 加入草稿；发送时把引用写进用户消息的**规范化引用块**，模型可用文件工具读取。
- **G3 附件**：拖放/文件选择 → 客户端校验大小/类型 → 通过**新增单一 IPC** 保存到 workspace 内附件目录（工具可读）→
  chip 展示名称/大小/类型/上传中/失败重试；发送时与 `@` 引用一起进入引用块；大小/数量/类型上限明确，失败不丢草稿。
- **G4 模型显示与用量**：composer 显示当前模型标签（personal = 设置里的 provider/model；enterprise = 已有 gateway model），
  点击打开「设置 → 模型」；显示本轮 usage（input/output/cached/total）与会话累计（持久化 usage 求和），
  数据只来自既有 `AgentUsage`/`usage` 事件，不新增 provider 调用、不伪造上下文窗口百分比。
- **G5 键盘/可达性/六态**：`/`、`@`、Shift+Enter、Esc、↑↓、Enter、IME 安全；菜单为 combobox/listbox 语义、
  焦点不离开输入框、关闭后焦点回输入框；空/加载/错误/离线/流式中/已取消六态不白屏；i18n 全量键（zh/en）与 axe 0 serious。

## 3. 非目标

- 不做 `/model` 内联切换、模型发现/多 provider 下拉/推理等级选择（本轮只读展示 + 跳设置；见 §7-3）。
- 不做 Host 命令（`/compact`/`/export`/`/feedback`/`/goal`/`/permission`/`/plan`）与 `/<skill>` 技能命令。
- 不做 `@` 会话引用 / 技能引用 / 符号（symbol）引用；本轮只做 workspace 文件。
- 不做附件原生多模态直传（图片字节直接进 provider）；附件 = workspace 文件 + 引用，模型用 `read_image`/`read_document` 读取。
- 不做 queue/steer（繁忙时排队/插话）、附件编辑/裁剪/灯箱、附件垃圾回收。
- 不注册新的 `ui.composer` slot（`docs/28` 明确 composer 留在 `ui-chat`；包内拆组件，不新增 slot/包）。
- 不改 `chat.send` 契约（引用/附件通过用户消息文本的规范化引用块表达）；不改 agent/preamble。
- 不引入运行期插件加载（C2）；不新增 provider 调用或网络请求。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 个人用户 | 在输入框打 `/` 想快速新建会话/打开设置 | 就地弹出命令列表，↑↓/Enter 执行，Esc 关闭且不丢草稿 |
| 个人用户 | 让 agent 看某个源码文件 | 打 `@` 选 workspace 文件，chip 可见可删；发送后模型能读取该文件 |
| 个人用户 | 附上一张截图/一个 PDF 作为上下文 | 拖放后看到 chip/缩略图/大小；超限/类型不支持时明确提示且草稿保留 |
| 个人用户 | 想知道当前模型与本轮消耗 | composer 底部显示模型标签与 token 用量；点击模型标签打开设置 |
| 企业用户 | 在网关模型下对话 | 模型标签显示已选 gateway model；不出现跨网关/本地 key 控件 |

## 5. 成功指标

- 键盘：`/` 弹出→过滤→Enter 执行→清空触发文本；`@` 过滤→Enter 加 chip；Esc 关闭且草稿不变；IME composing 时 Enter/`/`/`@` 不误触发（单测 + e2e）。
- 附件：≤1 MiB/文件、≤5 个/条消息、8 类 allowlist；非法类型/超限有可读错误且草稿保留；上传成功后文件可在 workspace 内被工具读取（host 单测 + e2e 经 IPC 断言）。
- 引用块：发送后用户消息持久化内容含规范化引用块；UI 展示为 chip（不展示原始块）；刷新后仍可见（e2e）。
- 命令目录单一来源：⌘K 与 `/` 菜单渲染同一数组/同一 `runCommand`；`rg` 不出现第二份命令常量（代码评审 + 单测）。
- 模型/用量：标签随设置/网关模型变化；usage 在流式/完成后更新；无 usage 时显示「用量未报告」而非 0 或伪造百分比。
- a11y：axe 0 serious/critical；combobox/listbox 语义、焦点管理、屏幕阅读器可用（e2e 断言）。
- i18n：`bash scripts/i18n-check.sh` exit 0；无硬编码中文/英文文案。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解/验证 |
| --- | --- | --- |
| 附件走 IPC base64，serve 载体 axum `Json` 默认 2 MiB body limit | 中 | 单文件上限取 **1 MiB**（base64+JSON <2 MiB，无需改 `/ipc` body limit）；host 解码后再校验；单测超限拒绝 + e2e 1 MiB 内成功；更大上限作为后续（需同改 serve body limit） |
| `.zeokeel/attachments` 写入 workspace 会出现在文件列表 | 低 | 文件树/`@` 候选默认隐藏 `.zeokeel`（UI 过滤）；附件路径由 chip 直接引用；`KNOWN-LIMITATIONS` 记录 |
| composer 触发键与 IME/中文输入冲突 | 中 | 只在不 composing 时处理；`compositionstart/end` + 单测（模拟 events）；`/`/`@` 仅在词首触发 |
| 原生受控 textarea 自管 IME/键盘/焦点 | 中 | 明确状态机（Enter/Shift+Enter/Esc/↑↓/composition）+ 单测 + e2e；不写富文本/第二套编辑器；保留现有容器外观 |
| 附件不经模型多模态，图片需工具读取 | 中 | 引用块明确列出路径；agent 已有 `read_image`/`read_document` 工具；`KNOWN-LIMITATIONS` 记录 |
| 1 MiB 附件 + base64 内存峰值 | 低 | 单文件并发上传上限 2；解码后立即校验大小；不缓存原始 base64 |
| 模型标签 enterprise/ personal 来源不同 | 低 | UiValue 注入单一 `modelLabel`（app 计算），ui-chat 不读 edition 分支 |
| 命令目录与 ⌘K 漂移 | 中 | 同一 catalog 常量 + 同一 `runCommand`；单测断言两处长度/id 集合一致 |

## 7. 已拍板决策（原待拍板项；用户 2026-11-01 批准推荐项）

- [x] **D1 命令集边界**（**已定（用户 2026-11-01 批准推荐项）**）：首期仅复用现有 client 命令目录：**当前 catalog 为 personal 4 条**（新建会话/设置/侧边栏/右栏），本 spec 新增 `cancel`/`retry` 后成为 personal 6 条；enterprise 再加技能/连接器/应用 3 条 = 7→9；
      不接 Host 命令与技能命令。**采用 A**；备选 B（加 `/skill`/`/compact`）会引入 Host 命令执行面与新错误链，超出 Phase 2a。
- [x] **D2 附件存储位置与上限**（**已定（用户 2026-11-01 批准推荐项）**）：**采用 A**：workspace 内 `.zeokeel/attachments/<conversation>/`（工具可读、随工作区走），
      单文件 ≤1 MiB、每条 ≤5 个、类型 allowlist（png/jpeg/webp/gif/pdf/txt/md/json/csv/log）；备选 B（5 MiB 上限 + `serve /ipc` body limit 8 MiB，+1 文件）留作后续，备选 C `$ZEOKEEL_HOME/attachments`（模型/工具不可读，不推荐）。
- [x] **D3 模型选择本轮范围**（**已定（用户 2026-11-01 批准推荐项）**）：**采用 A**：只读展示当前模型 + 点击打开设置并**定位到模型相关 pane**（personal `provider` / enterprise `enterprise`；pane 深链经既有 `SettingsPanes` id，新增可选 `openSettings(paneId)`，不做内联切换），不做内联切换/模型发现；
      备选 B：本轮做内联切换（需写 `settings.set` + revision 冲突处理 + 发现模型，约 +4 文件，建议拆 Phase 2a-2）。
- [x] **D4 context 计量口径**（**已定（用户 2026-11-01 批准推荐项）**）：**采用 A**：只显示既有 usage（本轮 input/output/cached/total + 会话累计），
      不显示 context window 百分比（prompt budget 未暴露，拒绝伪造分母）；备选 B：暴露 agent prompt budget 作为分母（需 agent/config 契约改动）。
- [x] **D5 `@` 与附件范围**（**已定（用户 2026-11-01 批准推荐项）**）：**采用 A**：`@` 只做 workspace 文件；附件只做 workspace 文件 + 引用块（不直传多模态）；
      备选 B：本轮加 `@` 会话引用/图片多模态直传（跨 agent/llm 契约，超预算）。
- [x] **D6 发送语义**（**已定（用户 2026-11-01 批准推荐项）**）：**采用 A**：引用/附件与文本合并为一条用户消息（引用块在文本尾部），允许「只有引用/附件、无文本」发送；
      备选 B：新增 `chat.send` 结构化 `references/attachments` 字段（需改契约与 host，+2 文件）。

> 已拍板（D1–D6 均采用推荐项）；状态仍为 `draft`，待独立评审通过后转 `approved`（C16/C19）。

## 8. 变更预算与拆分

- 预计改动文件（实现+文档，不含本 spec 目录/生成物）：
  契约 1（`packages/contracts/ipc/src/lib.rs`）；kit 1（`crates/zk-kit-tools/src/lib.rs` 新增 `safe_join_under`，单测同文件）；
  生成器源 1（`tools/zk/src/main.rs`：`zk-composed` 生成 facade，暴露 kit/tool 能力，**不手改 @generated 模板/生成物**）；
  desktop 1（`apps/desktop/src/ipc.rs`）；ui-kit 2（`types.ts`、`context.tsx`）；ui-chat 3（`AssistantComposer.tsx`、`AssistantThread.tsx`、`tests/composer.test.tsx`）；
  desktop-web 3（`App.tsx`、`i18n.tsx`、`packages/ui/ui-settings/web/src/index.tsx`（可选 initialPane））；e2e 1（`e2e/m1-serve.spec.ts` 追加）；
  文档 3（`docs/19`、`docs/26`（§5 IPC/错误码）、`KNOWN-LIMITATIONS.md`）。**合计 16**（Lead 批准的 Phase 2a 预算例外：原 15 + docs/26；kit helper/生成器 facade 替代原桌面/controller 项后仍为 16）。
- 生成物：`shared/contracts/{schema,ts}`、`apps/desktop/web/src/generated/*` 由生成器写（C11，不手改）。
- 如需 D3-B（内联模型切换）：+约 4 文件（settings 写入/revision、模型控件状态、i18n、测试），超出 16，
  按 §8 拆 `2026-11-01-client-composer-model-switch`；本 spec 只做 D3-A。
- 不拆 spec 的理由：16 文件围绕单一「输入面」目标，符合 Lead 的 Phase 2a 预算例外（16）；无第二命令目录、无第二编辑器。

## 9. 复用与已有实现（C17）

- 已搜索：`rg -n "AssistantComposer|composerPlaceholder|keyboardHint|workspace.files|AgentUsage|usage" packages apps docs`；
  `./target/debug/zk find kit.tools` → `crates/zk-kit-tools/src/lib.rs`；`zk find kit.jobs` → `crates/zk-kit-jobs/src/lib.rs`；
  capability 索引 `contract.ipc`（`shared/contracts/capability-index.json:43`）、`kit.tools`（`:131`）、`contract.config`。
- 可复用：
  - 命令目录 + 执行：`apps/desktop/web/src/App.tsx:591-607` 的 `commands`/`runCommand`（提升为唯一 catalog，供 ⌘K 与 `/` 共用）。
  - 全局面板键盘/搜索/a11y 模式：`packages/ui/ui-commands/web/src/index.tsx`（listbox/combobox、↑↓/Enter/Esc；composer 菜单复用同一交互模式，不复制组件状态）。
  - 文件候选：IPC `workspace.files`（`packages/contracts/session/src/lib.rs:100-124`、`apps/desktop/src/ipc.rs:914`）与 `parseFileEntries`（`apps/desktop/web/src/protocol.ts:164`）。
  - 用量：`AgentUsage`（`packages/contracts/agent/src/lib.rs`）、`usage` 事件、`UsagePart`/`readUsage`（`packages/ui/ui-chat/web/src/UsagePart.tsx`）。
  - 发送/停止/离线状态：`UiActions.send/cancel`、`UiValue.streaming/offline`（`packages/ui/ui-kit/web/src/context.tsx:37-80`）。
  - 设置持久化：`settings.get/set` + revision（settings-ia），模型标签读取 `AppSettings.provider` / 既有 gateway model。
  - 附件文件校验：复用 host 既有 workspace/conversation 归属校验（`chat_send` 的 `list_conversations_in`）；路径安全**复用新增的单一 helper
    `zk_kit_tools::safe_join_under(root, parts)`**（逐组件 no-follow、canonicalize 父目录、`starts_with(root)`、拒绝绝对/`..`；父目录 symlink 逃逸单测）；
    host 经 `zk-composed` 生成 facade 调用，**app 不直接依赖 kit/tool 实现**（C4/C2）。
- 本次新增可复用能力：`zk_kit_tools::safe_join_under`（`kit.tools` 内扩展，不新增 capability id）；新增 1 个 IPC 命令/DTO（`contract.ipc` 扩展）；不登记新 capability/INDEX。
- 为什么不能复用/扩展：composer 触发/附件/模型标签是展示与输入交互，现有 `ui-commands` 是全局 overlay 组件且状态由 app 持有，
  不能直接内嵌 composer；因此复用其**交互模式与同一命令目录**，组件与状态仍在 `ui-chat` 内（不新增 slot，符合 `docs/28` §4）。
