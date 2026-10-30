# 02 · 包模型：什么算一个"包"

> 本文件定义**唯一真源**：包的身份、清单、角色（role）、契约（contract）、基数（cardinality）。
> 组合器、生成器、CI 校验、文档索引全部从这里派生。
> 状态：Plan v3.2 · 与 `03-composition-and-ci.md`、`10-repo-and-build.md` 配套阅读。

## 1. 定义

**一个包 = 一个可独立版本化、可被组合、可被整体替换的实现单元。**

它必须同时满足四条，缺一不算包：

| 条件 | 说明 | 反例 |
| --- | --- | --- |
| 有清单 `zk.toml` | 身份、版本、提供的角色、依赖的角色、构建 facet | 一个 `src/utils.rs` 工具文件 |
| 有契约 | 对外只暴露契约 crate 里的 trait / DTO，不暴露自身类型 | 一个导出具体 struct、被别处 `use` 的业务模块 |
| 有符合性测试 | 实现了哪个契约，就跑哪个契约的 conformance 套件 | 只有"能跑通"的手测 |
| 可被替换 | 组合器把角色换到另一个包后，其余代码零改动 | 全局单例、`static` 全局状态、硬编码路径 |

**推论**：包不是"文件夹"，是"可替换边界"。一个包可以同时有 Rust facet 和前端 facet，
但**替换时两个 facet 一起换**——这是"UI 与能力同属一个包"的语义保证（见 §6）。

## 2. 包清单 `zk.toml`

每个包根目录一份，**唯一真源**。`Cargo.toml` / `package.json` 里的对应字段由 `zk` 校验一致
（而不是反过来），避免同一事实两处手写。

> `zk.toml` 的冻结字段/示例见 [`24` §2](24-composition-and-runtime-spec.md#2-zktoml-package-manifest-v1冻结字段)。本节只定义包模型语义（id/role/contract/requires/facets），不重复 manifest 字段。


设计约束：

- `id` 与 `version` 一旦发布不可变；替换发生在 profile 里，不在包内。
- `requires` 写**角色**不写包名。若写成包名，换掉那个包就编译不过，"一切皆包"失效。
- `[facets]` 声明实际构建单元；没有 `ui` 就没有前端产物，组合器据此决定是否进 pnpm 图。
- `[config].schema` 让用户配置与包解耦：换实现时配置校验规则跟着包走。
- `[permissions].declares` 不是插件安全边界（Rust 同进程静态链接，没有隔离可言），只用于
  统一"用户审批模式"与诊断面板；这一点与先例 D2 结论一致，别把它包装成沙箱。

## 3. 包的 kind（v3.2 扩展为八种）

| kind | 职责 | 允许提供 | 典型例子 | 可替换性要求 |
| --- | --- | --- | --- | --- |
| `contract` | 稳定缝：trait + DTO + 符合性套件 | 无（被依赖） | `zk-contract-tool`、`zk-contract-llm` | 同 major 内可换 mock/替代实现；跨 major = 新 crate。**必须发布并以 `[patch]` 锁到单实例**（ADR-016） |
| `runtime` | 宿主运行时：cordis 宿主、事件、配置、存储抽象、IPC | `host.*` | `zk-runtime`、`zk-ipc` | 可选替换（自研/精简宿主） |
| `platform` | 桌面/Web 平台适配 | `platform.*` | `zk-platform-tauri`、`zk-host-webserver` | 可替换（Tauri ↔ 纯 Web ↔ 未来 wry） |
| `capability` | 业务能力：工具、LLM provider、存储、技能、MCP | `tool.*`、`llm.*`、`store.*` | `tool-fs`、`llm-deepseek` | **核心替换对象** |
| `ui` | 前端组件、槽位、主题、品牌、i18n | `ui.*`、`theme.*`、`brand.*` | `ui-chat`、`ui-theme-default` | **核心替换对象** |
| `product` | 组合产物：桌面壳入口、服务端入口、管理台入口 | — | `apps/desktop`、`apps/server`、`apps/admin` | 不替换（是输出，不是输入） |
| ★`server` | 服务端域：路由/仓储/作业/迁移 | `srv.*`、`wallet.*`、`gateway.*` | `srv-auth-local`、`srv-wallet`、`srv-gateway` | **核心替换对象**（OEM 可替换整域） |
| ★`admin-ui` | 管理台页面与 UI 包 | `admin.page.*`、`admin.slot.*` | `admin-users`、`admin-gateway` | **核心替换对象** |

`product` 是**产物入口**的标记，不进入可组合包集合（它由 profile 决定包含哪些包）；单独列出是为了让"什么不是包"显式化。

**edition 与 OEM 不是 kind，是组合层**：`editions/personal.toml` 是基线；`editions/enterprise.toml`
`base_edition = "personal"` + `exclude` 个人专属包后**追加企业管控包 + 覆盖角色**（企业版 = (个人版 − 个人专属包) + 企业管控包）；`oem/<customer>/`
在 edition 之上叠加品牌/文案/配置（深度 OEM 可追加包）。见 `03` 与 `09`。

**关于"一切皆包"的边界（必须讲清，否则会被当成口号）**：

| 类别 | 可否替换 | 说明 |
| --- | --- | --- |
| `capability` / `ui` / `platform` / `runtime` / `server` / `admin-ui` | ✅ 可替换 | 改 profile 即可，CI 矩阵自动验证 |
| `contract` | ⚠️ 同 major 内可替换（mock/替代实现）；跨 major = 新契约 | 契约本身就是替换边界；换掉它等于换掉所有实现的对接面，所以需要显式升 major + 全生态对拍 |
| `product`（产物入口） | ❌ 不是组合输入 | 它是输出 |
| 引导代码（`main.rs`/`main.tsx`） | ❌ | 必须有不参与组合的入口（ADR-009） |

所以准确的说法是：**所有"实现类"包都可替换；契约是唯一稳定缝，它按 major 版本演进而不是被随意替换。**
v3 追加两类实现包：`server`（服务端域）与 `admin-ui`（管理台页面）。它们与客户端包**同规则**：
角色+契约+清单+conformance；服务端包额外带不可变迁移与路由权限声明（见 06）。
同理，`main.rs` 与 `main.tsx` 的引导代码不是包——**必须有一小段代码来创建 cordis root**，
这是唯一例外（ADR-009），且它被约束到"只做创建与注册，不含业务"。

## 4. 角色（role）与基数

角色是**替换的插槽**。命名采用 `<域>.<名字>`，稳定、可枚举、可文档化。

| 角色族 | 例子 | 基数 | 说明 |
| --- | --- | --- | --- |
| `tool.*` | `tool.fs`、`tool.shell`、`tool.todo`、`tool.web` | singleton（按名字唯一） | 每个工具名唯一；工具集合由框架聚合 |
| `tool.group.*` | `tool.group.search` | multi | 一个包可贡献一组工具 |
| `llm.*` | `llm.provider.default`（singleton）/ `llm.provider.<vendor>`（multi） | 见左 | M1 用一个兼容 OpenAI 的 provider 实现覆盖多家供应商（base_url + 模型目录）；需要多供应商并存时用 multi 角色，M3 起 |
| `agent.*` | `agent.loop`、`agent.prompt.section` | singleton / multi | 循环唯一，提示词片段可多 |
| `store.*` | `store.session`、`store.settings` | singleton | 持久化实现 |
| `host.*` | `host.fs-access`、`host.process`、`host.net`、`host.secrets` | singleton | 宿主能力，被 capability 依赖 |
| `platform.*` | `platform.shell`、`platform.ipc`、`platform.http` | singleton | 桌面壳 / IPC 传输 / serve 载体；换壳只改 profile |
| `ui.shell` / `ui.transport` | — | singleton | 外壳唯一；传输实现（tauri/http/mock）由 profile 选 |
| `ui.slot.<位置>` | `ui.slot.settings.section`、`ui.slot.composer.action` | multi | 槽位贡献 |
| `theme.*` / `brand.*` / `i18n.*` | `theme.tokens`、`brand.mark` | singleton | 换肤/白标的最小替换面 |
| ★`srv.auth` / `srv.users` / `srv.wallet` | 例：`srv.auth.source`、`srv.users`、`srv.wallet` | singleton | 服务端域，一域一实现；OEM 可整域替换（无部门/用户组） |
| ★`srv.gateway` / `srv.skills` / `srv.connectors` / `srv.apps` | — | singleton | 服务端内容/网关/应用域 |
| ★`admin.page.*` / `admin.slot.*` | `admin.page.users`、`admin.slot.nav` | singleton / multi | 管理台页面与槽位 |

基数语义：

- `singleton`：组合期必须**恰好一个**提供者；0 个 → 组合错误（除 `optional` 角色）；≥2 → 组合错误。
- `multi`：0..N，按 `order` 排序后注入同一注册表。

**只允许 singleton 在 profile 里被覆盖**；multi 角色靠"选哪些包"来增删，不做单点覆盖。
这条规则让 profile 的语义保持简单：`[packages]` 表是"角色 → 包 id"，重复即错误。

## 5. 契约（contract）：唯一的稳定缝

契约 crate 是这个架构里**唯一**允许跨包直接依赖的东西。规则：

1. 契约只含：trait、DTO（`serde`）、错误类型、常量、符合性套件（`conformance` 子 crate）。
2. 契约**不含**实现、不含 `rig`/`tauri` 的公开类型。cordis 类型**只允许**出现在 `zk-contract-kernel`
   （`Registrar` 门面）与 `zk-contract-package`（仅限 `Installed` 里的 `Arc<dyn Plugin>` 适配器）；
   领域契约（tool/llm/session/ui…）不得出现。
3. 一个契约 = 一个 crate + 一个 major 版本；`zk.contract.tool@1` 对应 crate `zk-contract-tool`
   的 `1.x`。major 变化 = 新 crate（`zk-contract-tool2`），不与旧版共存于同一 profile。
4. **每个契约 crate 只能有一个 package id**（name+version+source 三元组，不是"版本号"）。
   `zk validate` 用完整 `cargo metadata` 校验，且 **fail closed**：拿不到 metadata 就报错，不降级。
   原因：trait 身份由 crate 实例决定。评审实测三种骗过"按版本去重"的形态——同名不同 path、
   同名不同 git+rev、改名私有副本——都必须被拦下。
5. **契约 id ↔ crate 名显式登记**：根 `zk.toml` 的 `[contracts]` 表是唯一映射，profile 里出现的每个
   `zk.contract.*` 都必须在这里有同名 crate（`zk validate` 双向校验），避免"契约 id 写了但没有 crate"。
6. 契约变更流程：`cargo public-api` 差异检查 + 符合性套件跑全实现 + semver 决定是否升 major。
7. **服务端契约额外要求**：路由权限点、错误码、分页/幂等语义写进契约；迁移不属于契约（属于包），
   但迁移的**内容哈希**必须与包版本对应（见 06 §3）。

契约清单（初版，M0 定型）：

| 契约 crate | 关键 trait | 用到的包 |
| --- | --- | --- |
| `zk-contract-package` | `Package`（`descriptor()` / `install() -> Installed`，**不持有 Context**） | 全部 |
| `zk-contract-kernel` | `Registrar`（封装 `ctx.plugin`）、`Clock`、`Tasks`、`Config`、生命周期类型 | runtime/platform |
| `zk-contract-tool` | `Tool`（spec + call）、`ToolRegistry` | tool-* |
| `zk-contract-llm` | `LlmProvider`（chat/stream/capabilities）、`ModelRef` | llm-* |
| `zk-contract-agent` | `AgentLoop`、`Turn`、`ToolCall`、`ApprovalRequest` | agent-* |
| `zk-contract-session` | `SessionStore`、`Conversation`、`Message`、`Checkpoint` | store-*、agent-* |
| `zk-contract-host` | `FsAccess`、`ProcessRunner`、`NetClient`、`Secrets` | host-*、capability |
| `zk-contract-platform` | `ShellHost`（窗口/主题/生命周期）、`HttpHost`（bind/serve/token） | platform-* |
| `zk-contract-ui` | `UiContribution`、`SlotId`、`ComponentId`、`ThemeTokens`、`I18nBundle` | ui-* |
| `zk-contract-ipc` | `Command`、`Event`、`StreamChannel` | runtime、platform、ui |
| `zk-contract-config` | `ConfigSchema`、`ConfigStore` | runtime、capability |
| `zk-contract-server` | `ServerPackage`：`migrations()`/`jobs()`/`config_schema()`（路由由 manifest `[[routes]]` 静态声明） | srv-* |
| `zk-contract-api` | 客户端↔服务端 API DTO、错误模型、分页、版本 | 客户端/服务端 |
| `zk-contract-auth` | `AuthProvider`（local/OIDC/LDAP/…）、身份与角色映射 | srv-auth-* |
| `zk-contract-user` | 用户/角色仓储与查询（**无部门**） | srv-users、admin-ui |
| `zk-contract-wallet` | 余额、账本、价格表、扣费/充值 | srv-wallet、srv-gateway |
| `zk-contract-gateway` | 上游转发、限流、计量钩子 | srv-gateway |
| `zk-contract-skill` | 技能包规范、发布/审批/可见范围 | srv-skills、client-skill |
| `zk-contract-connector` | MCP 配置、令牌、启停 | srv-connectors、client-connector |
| `zk-contract-app` | WASM 应用清单、帧 ABI、生命周期、DB | srv-apps、client-wasm-host |
| `zk-contract-admin-ui` | 管理台页面注册、权限点、表单/表格约束 | admin-ui-* |
| `zk-contract-testkit` | 符合性套件宿主（非运行时依赖） | 测试 |

M0 只实现最小集：`contract-package`（含 `Registrar` 入口）+ `contract-kernel`（Registrar/Clock/Tasks/Config）
+ `contract-tool`。其余契约在 M1 按需增加；**每个 `requires` 里出现的契约 id 必须有 crate**。

`zk-contract-testkit` 不属于 `contract` kind，它是 `dev` 工具包；但同样"一个版本"。

## 6. Rust facet 与 UI facet 的绑定

一个包可以同时声明两个 facet。组合时**原子替换**：

```toml
# packages/client/tool-fs/zk.toml
[facets]
rust = { crate = "zk-tool-fs" }
ui   = { package = "@zeokeel/ui-tool-fs" }   # 该工具的设置面板/结果渲染
```

为什么绑定而不是拆成两个包：

- 换掉 `tool.fs` 实现时，它的 UI（参数表单、结果渲染、诊断）必须跟着换，否则 UI 假定旧实现的数据形状。
- 反过来，UI 包的 `componentId` 由 Rust 侧 `UiContribution` 声明，编译期就能校验"声明的组件在
  web 侧存在"，避免运行时白屏。
- 代价：纯 UI 包也必须带一个极小 Rust 声明 crate（几行 `UiContribution`）。这是**故意的**——
  它让"UI 也是包"在类型系统里成立，而不是靠约定。

纯 UI 包（主题、品牌、说明文档页）同样有声明 crate，内容可能只有 `theme.tokens` 的静态数据。

## 7. 版本与兼容策略

- 包版本：`0.x` 期间 minor 可以破坏，但**同一 profile 内必须满足 requires 的契约版本**。
- 契约版本：`1.x` 内只增不改；删除/改签名 → `2`。`zk validate` 拒绝"提供者声明的契约 major
  与消费者 requires 不匹配"。
- 内核版本：`zeokeel-cordis` 视为外部依赖，pin 到 `0.6.x`，升级单独提交（先例教训：内核升级要
  跑全量矩阵，见 `research/prior-art.md`）。
- `zk.lock`：记录每个 profile 解析出的包 id + 版本 + 源 + 内容哈希；CI `--locked`。换包 = 改
  profile + 更新 lock，diff 可审。

## 8. 反模式清单（写进 review checklist）

| 反模式 | 为什么破坏"一切皆包" |
| --- | --- |
| 包之间直接 `use` 对方的具体类型 | 换实现后编译失败；必须走契约 |
| 在包里写 `#[tauri::command]` | 命令表面变成静态注册，无法按包生成/裁剪；改为包声明命令 + 单一分发入口 |
| 包读环境变量/绝对路径决定行为 | 换实现时行为不可预测；改为 `host.*` 契约 + 配置注入 |
| 全局 `static`/`OnceCell` 单例 | 无法在测试里装两个实现，也无法隔离 profile |
| UI 包直接 import 另一个 UI 包的内部组件 | 换掉被依赖包后崩；只允许通过 `ui.slot` 与 `registry.component(id)` |
| 在 `Cargo.toml` 里用 feature 开关实现变体 | 组合爆炸 + feature 统一污染，且无法表达"角色/基数/UI facet"；用 profile（见 03 §3） |
| 服务端包直接 `Router::new()` 挂路由 | 路由不可枚举、权限矩阵不可生成；必须走 manifest `[[routes]]` 静态声明（见 24 §4） |
| 服务端包改写历史迁移 | 已发布库无法对齐；只允许追加前向修复（ADR-022） |
| 管理台页面直接调别的域 SQL/仓储 | 破坏可替换性；只能经 API/契约 |
