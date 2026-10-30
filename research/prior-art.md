# 先例研究：ZeoKeel-Next → ZeoKeel「一切皆包」

> **研究对象**：`/data/ZeoKeel-Next`（只读，未做任何修改）。
> **研究目标**：为 ZeoKeel 的「一切皆包、所有包能在 CI 阶段被替换、前端组件也是包」提取可复用资产与必须重做的部分。
> **快照**：`HEAD = 5e42c21`（2026-10-30 09:41:12 +0800）**+ 236 项未提交改动**（220 项修改/重命名 + 16 项未跟踪目录）。工作区正在做「生成式构建图 + UI 进插件包 + Root 贡献 + 新文档 wiki」的大迁移，**迁移本身尚未提交**：
> - `[HEAD]` = 手写注册表 + per-package Cargo feature + 旧扁平 docs；
> - `[工作区]` = 生成式 `plugin-bundle` + 生成式 server compose + UI 进包 + 编号 wiki（`scripts/gen-plugin-graph.py`、`scripts/gen-server-compose.py`、`client/crates/plugin-bundle/`、`client/web/src/ui/registry/modules.generated.ts`、`docs/00-overview…70-reference/`、`server/bin/zeokeel-server/tests/compose.rs` 全是**未跟踪新文件**）。
>
> 下文只用 `[工作区]` 标注「只在未提交工作区成立」的结论；未标注的实现细节在工作区可读，但**引用为「先例资产」前必须确认新项目要的是迁移前还是迁移后的形态**（两者在「per-package feature」这点上结论相反，见 §4.3）。
> **一句话判断**：ZeoKeel-Next 已经把「插件 = 构建期静态包」做到了 **80%**，但它验证的是 **"运行时禁用"**，不是 **"构建期替换"**；两字之差就是新项目全部工程量所在。

---

## 0. 一页结论

### 0.1 最该复用（Top 3，完整清单见 §9）

| # | 资产 | 为什么 | 路径 |
| --- | --- | --- | --- |
| 1 | **UI 贡献模型 + Root 即贡献**（`Root` 变体为 `[工作区]` 新增） | 「组件也是包贡献」的契约已经跑通并有真实 e2e；`UiContribution`/`UiSlotContribution` 是你能直接拿去当「包清单 UI 段」的唯一样本 | `client/crates/plugin/src/contract/manifest.rs:159-254`（HEAD 已有 `UiKind`/`UiSlotContribution`）；`client/web/src/ui/host/**`；`client/web/src/App.tsx`；`client/modules/features/plugin-core-ui/src/lib.rs` |
| 2 | **生成器范式：目录+清单发现 → 确定性排序 → 三个静态产物 → `--check` 精确字符串比对 → 生成物 vs 运行期真相双向交叉测试** `[工作区]` | 这是「包解析器 + 组合代码生成器」的现成骨架；server 侧还多一个「生成清单 ↔ inventory manifest 双向校验」的最强判据 | `scripts/gen-plugin-graph.py`；`scripts/gen-server-compose.py`；`server/bin/zeokeel-server/tests/compose.rs`；`client/crates/plugin-bundle/tests/bundle.rs` |
| 3 | **plugin-matrix 的真实 IPC 矩阵方法论** | 用「删掉的恰好等于它声明的」这一句式验证贡献归属，正是「替换」验证的雏形；harness（Xvfb+dbus+一次性 HOME+静态页+经真实 IPC 写报告）可整体搬走 | `scripts/plugin-matrix.sh`（544 行，含内嵌 JS 矩阵与 Python 判据） |

### 0.2 最该重做（Top 3，完整清单见 §10）

| # | 必须重做 | 为什么（今天的事实） |
| --- | --- | --- |
| 1 | **包清单模型（module.toml v2）** | 今天的 `module.toml` 只是「构建索引」，**不含**版本、依赖、导出、`provides/requires`、组件闭包、锁或哈希；它由 Cargo.toml/module.json **派生**，本身不是可解析的包契约（`scripts/gen-module-manifests.py`）。且一个包可能有 **1+N 个** module.toml（`plugin-core-ui` = 1 个 crate 级 + 7 个 ui 级），「一包一清单」不成立 |
| 2 | **构建图：从「全量聚合包」改为「可选子集 + 变体构建」** | `[工作区]` 的 `plugin-bundle` 聚合全部 32 个包、**没有 per-package Cargo feature**；而 `[HEAD]` 恰恰有（`plugins-default` + 32 个 `plugin-*` feature + 手写表）。迁移把「可裁剪」换成了「免手写」，两头都没同时拿到（`client/src-tauri/Cargo.toml` 的 `[features]`，HEAD 版） |
| 3 | **组件 id 闭包校验（真·构建期）** | 今天三份手写列表互相漂移：Rust 声明 22 个 id、`module.json` 注册 16 个、测试 vocabulary 16 个（含已不存在的 `core.host-sync`、缺 `core.user-question`）；**10 个已声明 component id 无人注册**（`files-page`/`mcp-page`/…），只在运行期降级成诊断卡。所谓「codegen 期失败」并未实现（`docs/10-architecture/module-packages.md:35` 是目标而非现状；实测见 `.review/evidence/20261030/auditor/10-ui-component-and-target-gaps.txt`） |

### 0.3 最大的隐性成本（完整见 §11）

| 成本 | 量化 | 说明 |
| --- | --- | --- |
| **验证经济学** | 一次客户端构建 ≈10 分钟（`ci.yml` 注释自述「building the app twice would waste ~10 minutes」）；32 个包 × 每包一次「缺席构建」= 5+ 小时 | 替换性必须用「N 个变体构建 + 快照 diff」证明，但今天**没有便宜的 headless 判据**；GUI 矩阵每次 2 次启动/90s 超时，只能证明 toggle。这是新项目第一优先要设计的东西，不是最后补的 |
| **四份拷贝的收敛** | 身份/版本/工具/组件今天存在于 Cargo.toml、`module.json`、Rust `PluginManifest` const、生成的 `module.toml`；每加一个字段 = 4 处改 + 3 个漂移检查 | 迁移 32 前端包 + 7 UI 包 + 13 服务端包 = 52 个清单；必须一次性 codemod，否则手工迁移本身就是新的漂移源 |
| **顺序语义藏在代码里** | 注册顺序是生成器里的 `PIN_FIRST`/`ORDER_AFTER`（`scripts/gen-plugin-graph.py:67-72`），并且影响 system-prompt 片段顺序 | 一旦「包可替换/可换序」，顺序必须变成**包清单里的数据**，否则每次替换都要改生成器代码 |

---

## 1. 先例快照（可复现）

| 维度 | 值 | 证据/命令 |
| --- | --- | --- |
| 客户端插件包 | 32（tools 10 / capabilities 6 / loaders 2 / features 14） | `ls -d client/modules/*/*/`；`docs/70-reference/plugin-catalog.md` |
| 工具 / 能力 | 73 / 41 | `./scripts/doc-metrics.sh` |
| IPC 命令 | 113（typed Tauri command，插件不可新增） | `shared/contracts/ipc-commands.json`；`docs/20-client/ipc.md` |
| UI 模块 | 7（**已搬进 `plugin-core-ui/ui/`** `[工作区]`） | `client/modules/features/plugin-core-ui/ui/*/module.tsx` |
| `module.toml` | 52 = 32 crate 级 + 7 UI 级 + 13 服务端 | `find . -name module.toml`；`docs/MODULE-INDEX.md` |
| 服务端模块 / 路由 | 13 / 266 | `cd server && cargo run -p zeokeel-server -- --dump-modules` |
| 规范化 slot | 41 = 38 视觉 + 3 数据待接（`app.theme.tokens`/`app.i18n.messages`/`app.keybinding`） | `client/web/src/ui/host/slots.ts`；`docs/UI-SLOTS.md` |
| 测试基线 | Rust attr 1025（实测 1026）/ 38 crate；Playwright 126 用例 / 14 spec；矩阵 32/32 插件 · 342/342 检查 | `docs/70-reference/metrics-baseline.md`；`./scripts/plugin-matrix.sh` |
| 代码规模 | client Rust 99,227 行；web ts/tsx 21,673 行；server Rust 138,131 行；docs 3,401 行；scripts 5,127 行 | `wc -l` 实测 |
| 内核 | `zeokeel-cordis 0.6.0`（外部仓库，466 tests / 481 with supervision；产品不跑内核测试） | `docs/10-architecture/cordis-runtime.md` |
| 仓库状态 | HEAD 干净可构建；工作区 236 项未提交（生成器 2/4 是未跟踪新文件，`plugin-bundle`/`modules.generated.ts`/新 wiki 全部未提交） | `git status --porcelain`；`git log -1` |

> ⚠️ 先例的**文档系统性滞后于代码**，而且是在同一次迁移内部就滞后：`docs/00-overview/target-vs-current.md`（本身是未跟踪新文件）把「UI 进插件包」「App.tsx 瘦身」「core.root」标为 ⏳，但同一次工作区改动里这些已做完；`AGENTS.md:54` 仍写「UI 包迁移完成前仍用 `import.meta.glob`」，而 `client/web/src/ui/registry/modules.ts` 已改成生成器静态组合；`target-vs-current.md:38` 还写「all 32 plugins are Cargo features linked into zeokeel」，而工作区已经删掉 per-plugin feature。另有 HEAD 版 `linked_modules.rs` 注释指向**不存在的** `contracts/codegen/gen_modules.py`。**不要按文档判断现状，按代码**（这正是 §8 的第 6 条教训）。

---

## 2. 目录与构建系统（问题 1）

### 2.1 分区职责

| 分区 | 职责 | 关键事实 |
| --- | --- | --- |
| `client/src-tauri/`（crate `zeokeel`） | Tauri 窗口/IPC/进程生命周期 + **仍是业务大头**（`agent/ data/ security/ features/`） | 平台与业务未分离；`target-vs-current.md` 唯一 ⏳ 的硬缺口 |
| `client/crates/plugin` | Cordis **宿主**：`FiberAdapter`、`PluginRegistry`、manifest、tools、hooks、UI 聚合、host services | 单 Cordis root/进程；`#![forbid(unsafe_code)]` |
| `client/crates/plugin-bundle` | **生成**的聚合 crate：依赖全部 32 个插件包，`register_all()` | 32 个 `path` 依赖 + 生成注册表；无 per-package feature |
| `client/modules/{tools,capabilities,loaders,features}/<crate>/` | 插件包（Rust；`plugin-core-ui` 额外含 `ui/`） | 一个 crate = 一个包 = 一个 fiber |
| `client/web/src/` | 贡献渲染器 + 薄 bootstrap（`main.tsx`/`App.tsx`）+ host/registry/primitives | **无第二个前端内核**、无运行时 JS 加载 |
| `server/` | 独立 Cargo workspace：薄 host + `zeokeel-kernel` + 13 模块（`inventory` 收集） | 客户端/服务端不共享 build；`tier` 只是启动失败策略 |
| `shared/contracts/` | 两端机器可读契约（生成） | `ipc-commands.json` / `ui-contributions.json` / `ui-slots.json` |
| `scripts/` | 跨端入口：4 个生成器 + 证据脚本 + CI 本地等价 | 见 §4 |
| `.review/` | 证据归档（gitignored）：auditor / e2e / 审计脚本 | `docs/50-testing/strategy.md` 的 L0–L3 政策 |

### 2.2 Cargo workspace

- 客户端：`client/Cargo.toml` 成员 = `src-tauri` + `crates/{plugin,plugin-bundle,cordis-loader,cordis-timer,dsh-port}` + `modules/*/*`（32），共 38 crate。
  - `[HEAD]`：`src-tauri` 直接持有 32 个 **optional path dep** + 32 个 per-plugin Cargo feature（`plugins-default` 全开）+ `features/plugins.rs` **手写 32 行注册表**（`#[cfg(feature = "plugin-*")] registry.register(...)`）。→ **可裁剪子集构建**，但每加一个包要改 3 处（Cargo deps/features/注册表）。
  - `[工作区]`：`src-tauri` **只依赖 plugin-bundle 一个 crate**，plugin-bundle 依赖全部 32 包并 `register_all()`；**per-package feature 已删除**。→ 免手写，但**全量绑定**（`docs/40-build/build-system.md` §Feature model、`plugin-bundle/src/lib.rs`）。
  - 新项目必须同时拿到两者：**生成的选择表 + 生成的 per-package feature**（§10-3、§12.2）。
- 服务端：`server/Cargo.toml` 成员由生成器写入 marker 块（`# BEGIN GENERATED MODULE MEMBERS`）；模块之间**零 Cargo 依赖**，只走 services/events（`docs/30-server/module-contract.md`）。`[HEAD]` 的 `linked_modules.rs` 是手写 13 行 `#[cfg(feature="module-*")] use ... as _;`（注释却声称由不存在的 `contracts/codegen/gen_modules.py` 维护）；`[工作区]` 改为 `include!` 生成文件。
- 客户端 `rust-version` 与 CI 使用 `--locked`；`cargo check -p zeokeel` 必须 0 warning（`-D warnings`）。

### 2.3 UI 模块如何注册（当前 `[工作区]`，这是「前端也是包」的现有答案）

```text
client/modules/<group>/<crate>/ui/<id>/module.json   # 元数据（id/kind/rustCrates/components/e2e）
        │  gen-module-manifests.py
        ▼
   …/ui/<id>/module.toml                             # 生成索引（52 个之一）
        │  gen-plugin-graph.py
        ▼
client/web/src/ui/registry/modules.generated.ts      # 静态 import + entries + modules[]
        │  modules.ts 只做 register()（+ 重复 id 断言）
        ▼
registerComponent(id, C)  ← Rust 侧 UiContribution.component 字符串在这里被解析
```

关键点：**没有 `import.meta.glob`、没有运行时加载、组件 id 是唯一跨语言契约**；`module.tsx` 必须导出 `meta` + `register()`，否则生成文件 **type-check 失败**（`modules.generated.ts` 的 doc 注释）。`[HEAD]` 版本用 `import.meta.glob`，`[工作区]` 已替换——这是一次值得新项目直接跨过的返工（见 §8-3）。

---

## 3. 插件契约（问题 2）

### 3.1 `module.toml` 精确字段（实测三类）

| 字段 | client Rust 包 | client UI 模块 | server 模块 | 来源 |
| --- | --- | --- | --- | --- |
| `schema` | `1` | `1` | `1` | 固定 |
| `id` | crate 名 | `module.json#id` | `ModuleManifest.id` | |
| `target` | `client` | `client` | `server` | |
| `kind` | `rust` | `fullstack` / `frontend` / `ui` | `server` | |
| `title` | crate 名 | `module.json#title` | `ModuleManifest.id` | |
| `crate` | Cargo package | — | `zeokeel-mod-*` | Cargo.toml |
| `group` | 目录名（tools/capabilities/loaders/features） | 目录名 | — | |
| `plugin` | — | **所属 crate 名** | — | Cargo.toml |
| `ui` | — | **入口路径（相对 crate）** | — | `module.tsx` |
| `crates` | — | `rustCrates` 列表 | — | module.json |
| `components` | — | **拥有的 component id** | — | module.json |
| `e2e` | — | Playwright spec 文件名 | — | module.json |
| `module` / `feature` / `tier` | — | — | `ModuleManifest.id` / Cargo feature / `base|optional` | src/lib.rs MANIFEST |

实例（三行看完差别）：

```toml
# client/modules/tools/tool-time/module.toml
schema = 1
id = "tool-time"
target = "client"
kind = "rust"
title = "tool-time"
crate = "tool-time"
group = "tools"
```

```toml
# client/modules/features/plugin-core-ui/ui/core-shell/module.toml
schema = 1
id = "core-shell"
target = "client"
kind = "fullstack"
title = "侧栏与命令面板"
plugin = "plugin-core-ui"
group = "features"
ui = "ui/core-shell/module.tsx"
crates = ["plugin-core-ui"]
components = ["core.sidebar", "core.commands"]
e2e = ["v2-shell.spec.ts", "v2-sidebar-extra.spec.ts", "v2-commands-presets.spec.ts", "v2-touch.spec.ts"]
```

```toml
# server/modules/zeokeel-mod-<id>/module.toml（字段名与 client 不同）
schema = 1
id = "<id>"
target = "server"
kind = "server"
title = "<id>"
crate = "zeokeel-mod-<id>"
module = "<id>"
feature = "module-<id>"
tier = "base"   # 或 optional —— 只是启动失败策略
```

**对「包清单」的直接启示**（详见 §12.1）：这是「构建索引」，不是「包契约」。缺 `version`、`dependencies`（包级）、`exports`（组件/槽/工具/服务对外面）、`provides/requires`、`effects/permissions`、`artifact hash`、`replaces`。字段还按 target 分裂成三套（client/server 不统一），无法用一个解析器统一消费。

### 3.2 `PluginManifest`（静态声明）

来源：`client/crates/plugin/src/contract/manifest.rs:284`。`'static`，直接序列化给前端。

| 字段 | 说明 |
| --- | --- |
| `id` / `name` / `version` / `description` | 稳定身份（`tool-fs`、`plugin-core-ui`…） |
| `kind` | `Tool` / `Capability` / `Provider` / `Loader` / `Ui` / `Composite`（**不是权限分层**） |
| `permissions` | 粗粒度声明 `fs_read/fs_write/shell/network/clipboard/model/system`；与**全局用户模式**求交，不是每插件授权 |
| `tools` | `ToolDecl { name,title,description,category,risk }` |
| `capabilities` | 能力矩阵行 `CapabilityDecl { id,group,name,status,module,feature,note }` |
| `ui` | `UiContribution[]`（整个表面） |
| `default_enabled` | 新装默认启用 |

派生：`PluginManifest::required()` = 「恰好一个 `UiKind::Root` 贡献」推导（`:377`）——`required` 是**启动失败策略**而非权限；`ToolDecl::read_only()` 由 `risk`+`category` 推导（`:314`，后来踩坑，见 §8-5）。

### 3.3 `Plugin` trait（行为）

来源：`client/crates/plugin/src/runtime/registry.rs:29`，`#[async_trait] trait Plugin: Send + Sync`：

| 方法 | 默认 | 作用 |
| --- | --- | --- |
| `manifest() -> &'static PluginManifest` | 必须 | 静态声明 |
| `tools() -> Vec<Arc<dyn ToolHandler>>` | 空 | 工具处理器（**每个运行期工具必须已在 manifest 声明**，registry 校验子集） |
| `hooks() -> Vec<Arc<dyn PluginHook>>` | 空 | 生命周期/tool hook |
| `tool_timeout(&str) -> Option<Duration>` | None | 每工具超时覆盖 |
| `context_budget() -> Option<ContextBudget>` | None | 压缩预算 |
| `prompt_sections() -> Vec<PromptSection>` | 空 | 有序 system-prompt 段（`name/order/text`，host 插值 `{{workspace}}`/`{{date}}`） |
| `commands()` / `command(&RpcContext, &str, Value)` | 空/拒 | 用户可调用命令（模型永不可见） |
| `ui_slots() -> Vec<UiSlotContribution>` | 空 | slot 片段 |
| `init(&PluginContext) -> Result<(), PluginError>` | Ok | 缺系统依赖则拒绝注册 |
| `preamble(&WorkspaceInfo, Option<&Path>)` | None | 每工作区 prompt 片段 |
| `on_disabled()` | 空 | 释放 OS 资源（PTY/进程/连接） |
| `attach(&RpcContext)` | 空 | 缓存 host services |
| `rpc(&RpcContext, &str, Value)` | 拒 | 插件自有 UI 调用（`plugin_rpc`） |
| `rpc_risk(&str) -> ToolRisk` | Medium | 审批/计划模式风险 |
| `dynamic_tools(&RpcContext)` | 空 | 运行期发现（MCP 每 server 工具一个） |

生命周期：`register(plugin, host)` → 校验 manifest（id 唯一、tool 名 ⊆ manifest、component id known）→ 建一条 Cordis fiber → 把 `on_disabled/tools/hooks/UI` 注册为 effect → `init(ctx)`；disable = dispose fiber + `on_disabled`，effect LIFO 回收（`docs/10-architecture/plugin-contract.md`）。

### 3.4 UI 贡献形状（**新项目「前端组件也是包」的直接起点**）

`client/crates/plugin/src/contract/manifest.rs:190` `UiContribution`：

| 字段 | 含义 |
| --- | --- |
| `kind` | `Root` / `Page` / `SettingsPanel` / `ToolRenderer` / `Command` / `CapabilityCard` |
| `id` | 表面 id（`chat`、`general`、`root`…） |
| `title` / `icon` | 展示；icon 为 Phosphor kebab-case，`""`=无 |
| `route` | 前端路由 id（shell 解析） |
| `order` / `group` | 导航组内顺序 / 导航组 id（`workspace`/`extensions`/`settings`） |
| `component` | **组件注册表 id**（`core.root`、`core.chat`…）——插件永不出 JS |
| `methods` | 该面板可调的 `plugin_rpc` 方法名 |

`manifest.rs:232` `UiSlotContribution`：

| 字段 | 含义 |
| --- | --- |
| `slot` | 41 个规范 slot 名之一；**未知名 = boot 审计错误**，绝不静默 |
| `id` | `(plugin, slot)` 内唯一 |
| `title` / `icon` / `order` | 同前 |
| `component` | 组件注册表 id |
| `props` | **不透明 JSON 对象字符串**（`""`=`{}`），Rust 永不解释 |
| `methods` | 允许调用的 `plugin_rpc` 方法 |

Slot 语义：`single` 取 `order` 最小；`list` 按 order→id；`keyed` 按 id 查。Host 覆盖：`PageHost/OverlayHost/SettingsHost/CommandPaletteHost/ToolRendererHost/CapabilityCardHost` + `ContributionSlot`（`docs/10-architecture/ui-contribution-model.md`）。

**Root 即贡献**（`[工作区]` 已实现，这是关键模式）：

```text
plugin-core-ui 声明 UiKind::Root / id="root" / component="core.root"
  → ui_contributions() IPC（只返回 enabled 插件）
  → App.tsx 只装 provider + 解析 root + 可见 fallback（loading/missing 两态）
  → makeContributionComponent 在组件缺失时渲染诊断卡（不静默）
```

证据：`client/web/src/App.tsx`（**零业务 import**）；`client/modules/features/plugin-core-ui/src/lib.rs`（51 条 slot + 24 条 whole-surface）；`client/web/src/ui/host/audit.ts`（`missingComponents`）。

### 3.5 契约的「三/四份拷贝」现状（必须承认的事实）

| 真相位置 | 内容 | 漂移实测 |
| --- | --- | --- |
| Rust `PluginManifest` const + `slot(...)` 表 | id、version、tools、components、slots | 声明 22 个 component id |
| `module.json`（每 UI 模块） | id/title/components/e2e/rustCrates | 注册 16 个 |
| `module.toml`（生成） | 索引 | 派生自上面两者 |
| `plugin-core-ui/tests/contract.rs:68` `COMPONENT_VOCABULARY` | 「必须存在」的 id 白名单 | 16 个：**含已不存在的 `core.host-sync`，缺 `core.user-question`** |
| `shared/contracts/ui-contributions.json#component_ids` | 生成契约 | 18 个：**miss 掉 8 个通过 `slot(...)` helper 声明的 id**（regex 只扫 `component:` 字面量） |

结论：今天有 **5 个地方** 各自维护 component id，且**三三互相不一致**；10 个声明的 id（`files-page`/`mcp-page`/`terminal-page`…）根本没有任何 React 实现，只靠「data-only contribution」这条宽容条款活着。新项目的包清单必须把「**导出面（exports）**」变成一等公民并机器校验闭包（§12.2）。

---

## 4. 生成器四件套（问题 3）

### 4.1 真实输入/输出

> 版本状态：`gen-module-manifests.py`、`ui-slot-report.py`、`export-contracts.sh` 在 `[HEAD]` 已存在；**`gen-plugin-graph.py`、`gen-server-compose.py` 是 `[工作区]` 新增（未跟踪）**。下面按工作区形态描述（这是新项目真正要借鉴的形态）。

| 脚本 | 输入（真相） | 输出 | 防漂移 | 守卫/判据 |
| --- | --- | --- | --- | --- |
| `scripts/gen-module-manifests.py`（7.2 KB） `[HEAD]` | client `Cargo.toml`（crate 名/目录组）；client UI `ui/<id>/module.json`；server `Cargo.toml` + `src/lib.rs` 的 `MANIFEST` | 每模块 `module.toml`（52）+ `docs/MODULE-INDEX.md` | `--check` 逐字节比字符串 | ① 旧 UI 树存在即 `SystemExit`（`LEGACY_CLIENT_UI`，`:28/:98`）→ 防「迁移回退」；② `module.json#id` 必须 == 目录名；③ `ui/` 必须有属主 `Cargo.toml` |
| `scripts/gen-plugin-graph.py`（18.7 KB） `[工作区]` | `client/modules/*/*/module.toml`（只收 `target=client, kind=rust`）+ `Cargo.toml` 包名 + `src/lib.rs` 存在性 + `ui/**/module.tsx` 与其 sibling `module.json` | ① `client/crates/plugin-bundle/Cargo.toml` ② `plugin-bundle/src/lib.rs`（`PLUGIN_MODULE_IDS`、`register_all()`、`plugin(id)`、reexports）③ `client/web/src/ui/registry/modules.generated.ts`（静态 import + entries + modules） | `--check` 三文件逐字节 | ① crate 名与 Cargo.toml 不一致即 fail；② 重复 crate/重复 UI module id fail；③ `ORDER_AFTER` 指向未知包/成环 fail；④ 注册顺序策略 `PIN_FIRST`/`ORDER_AFTER`（`:67-72`），`planned_order()` 拓扑（`:167`） |
| `scripts/gen-server-compose.py`（18.5 KB） `[工作区]` | `server/modules/*/Cargo.toml` 的 `[package.metadata.zeokeel]`（id / feature / failure_policy） | ① `server/Cargo.toml` members+workspace deps ② `bin/zeokeel-server/Cargo.toml` optional deps + `[features]`（default / `module-*` / `base-modules`）③ `linked_modules.generated.rs`（强制链接 + compose 清单 + `LINKED_MODULE_COUNT`）④ `server/README.md` compose 表 | `--check` 整文件 + marker 块比对；带 `difflib` 展示 diff | **marker 块拼接**（手写文件里只重写 `BEGIN/END` 之间，`:56/:323`）；`ID_RE/FEATURE_RE/CRATE_RE/POLICIES` 正则校验；缺 metadata 字段直接报错 |
| `scripts/ui-slot-report.py`（4.4 KB） | `client/web/src/ui/host/slots.ts` 的 `SLOT_DEFS` + 静态扫描 `<Slot name>`/`useSlotContributions` 宿主（扫 `web/src`、`modules`、`src-tauri/src`） | `shared/contracts/ui-slots.json` + `docs/UI-SLOTS.md` | ⚠️ **`--check` 只对「缺失宿主」失败，且脚本总是先写文件** → 不是漂移检查 | `DATA_SLOTS` 白名单 + `HOST_COMPONENTS` 别名表（`app.main`→`PageHost` 等） |
| （第五件）`scripts/export-contracts.sh` | 正则扫 `#[tauri::command]`、`manifest.rs` 的 `UiKind`、`slots.ts` 的 `SLOT_DEFS`、`client/modules/*/*/src/lib.rs` 的 `component:`/常量 | `ipc-commands.json`、`ui-contributions.json`、`ui-slots.json` | ❌ **没有 `--check`**（只能重跑后看 git diff） | 只扫 `src/lib.rs`；`component:` 字面量 + 同文件常量；**miss 掉 `slot(...)` helper 实参** → component 清单不完整 |

### 4.2 可复用的机制（这些是「包解析器 + 组合代码生成器」的现成零件）

| 机制 | 为什么值得抄 | 出处 |
| --- | --- | --- |
| **目录+清单发现，无中心列表** | 加包 = 加目录 + 跑生成器；删除包 = 删目录 + 跑生成器。这是「一切皆包」的最小实现 | `gen-plugin-graph.py:123 discover_plugins()` |
| **确定性排序 = 数据（策略表）+ 拓扑** | 文件系统顺序不可依赖；语义顺序（core-ui 先、skills→rules→memory）显式声明 | `:67-72`、`:167-190` |
| **生成物签入仓库 + `--check` 精确比对** | 新 clone 无需 Python 即可构建；CI 用 `--check` 拦住「手改生成物」 | `:422 write_or_check()` |
| **marker 块拼接（改手写文件的一部分）** | 这是升级到「往真实 Cargo/README/JSON 里注入生成段」的关键技巧 | `gen-server-compose.py:56,323` |
| **生成清单 ↔ 运行期真相 双向交叉测试** | 生成物说 A，`inventory` 说 B，测试直接失败；反向再查「运行期有但清单没有」 | `server/bin/zeokeel-server/tests/compose.rs`；`client/crates/plugin-bundle/tests/bundle.rs` |
| **生成物行为测试（不只比对文本）** | 断言「每个 id 真能注册、id 唯一、顺序语义成立」，比 `--check` 更强 | `plugin-bundle/tests/bundle.rs`（`registration_order_keeps_ui_and_preamble_priority`） |
| **shell 外置一致性** | `[package.metadata.zeokeel]` 让「包自己的 Cargo.toml」成为单一真源，生成器只做投影 | `gen-server-compose.py:4` |

### 4.3 为什么今天这套还**不是**「包解析器」/「组合代码生成器」

| 缺口 | 今天的事实 | 对「CI 可替换」的后果 |
| --- | --- | --- |
| 没有**依赖/版本/约束**字段 | module.toml 无 `dependencies`、无 `version`（版本在 Cargo.toml）；包间唯一关系是「都依赖 `plugin` crate」 | 无法检查「替换包 A 是否破坏依赖它的包」；无拓扑可解 |
| 没有**导出面（exports）** | 组件 id 只以字符串散落在 Rust/TS/测试/vocabulary；无「包 P 导出组件集合 S」的声明 | 无法做闭包校验；替换包时不知道谁提供 `core.root` |
| 没有 `provides/requires`（服务级） | 客户端靠 `PluginContext` 全局服务；服务端有 `depends_on/provides/requires`（inventory 只用于诊断） | 无法拒绝「缺提供者」的构建；运行期才 pending/failed |
| **发现 ≠ 解析** | 只做「glob + 读字段」，不建图、不求解、不校验边 | 加法容易，减法/替换没有判据 |
| **全量聚合 vs 可裁剪** | `[工作区]` `plugin-bundle` 依赖全部 32 包且**删除了 per-package feature**；`[HEAD]` 相反：32 个 per-plugin feature + `plugins-default`（可裁剪子集），但注册表手写 | 工作区形态无法 `--features minimal` 构建；HEAD 形态能裁剪但每包改 3 处。**两种形态都不足以支撑替换矩阵**，需要「生成的选择表 + 生成的 per-package feature」 |
| 一个包 **1+N 个清单** | crate 级 module.toml + 每 UI 模块一个 module.toml | 无法定义「一个包的完整边界」；删包要删多处 |
| 生成器 CLI 不统一 | `--check` 有/无/语义不同（ui-slot-report 假 check；export-contracts 无 check） | CI 无法「统一 check 所有生成物」；漂移漏网 |
| 生成物未记录**输入指纹** | 无 lock/hash/`built_at`/`generator_version` | 无法证明「同样包集 → 同样产物」，可复现构建证据缺失 |

---

## 5. `plugin-matrix.sh` 方法论（问题 4）

> 这是先例里最接近「CI 阶段替换验证」的东西。544 行，`scripts/plugin-matrix.sh`。

### 5.1 怎么发现插件

- **不硬编码清单**：先启动真应用，用真实 IPC `plugins_list` 拿到 `{plugins: [{id, kind, enabled, tools, ui}], tools}`，再遍历 —— 新插件自动纳入（`docs/50-testing/e2e.md`：脚本 auto-discovers，never hard-code the count）。
- `--only <id,id>` 过滤；`--no-build` 复用二进制；`--json` 机器可读（进度走 stderr，保 stdout 纯净）。

### 5.2 执行环境（真应用，不用 Playwright）

| 组件 | 做法 |
| --- | --- |
| 显示 | `Xvfb :90..:160`（自动找空闲）+ `openbox`；`MATRIX_DISPLAY` 覆盖 |
| 会话 | `dbus-run-session -- "$BIN"`；`GDK_BACKEND=x11`、`LIBGL_ALWAYS_SOFTWARE=1`、`WEBKIT_DISABLE_*` 一串 headless WebKit 适配 |
| 一次性 HOME | `$RUN/home/{config.json,workspaces.json}`；`ZEOKEEL_HOME`/`XDG_*` 全部指向 `$RUN`；工作区 `ws1` 含 `notes.md` |
| 假模型 | `config.json.llm.base_url = http://127.0.0.1:9/v1`（不可达）+ `auto_approve_tools=true` |
| 前端 | **不用 Vite**：`python3 -m http.server` 服务一个纯 HTML+JS 页面，`devUrl` 指向它；页面直接 `window.__TAURI_INTERNALS__.invoke` |
| 报告通道 | 页面用**真实 IPC 工具** `file_write` 把 JSON 写进工作区 `.selftest/report.json`，宿主轮询该文件（没有第二条旁路） |
| 端口 | 读 `tauri.conf.json#devUrl` 端口；被占则挑 `14320-14399` 空闲口 + `TAURI_CONFIG` **重编一次**并在退出时还原二进制（`:159-183`；注释承认早先靠 `strings` 猜端口会 flake） |

### 5.3 断言（逐条，「恰好」是关键）

对 **每个** 插件 `id`：

| 方向 | 断言 | 语义 |
| --- | --- | --- |
| disable | `plugin_set_enabled(id,false)` 调用成功 | 控制面可用 |
| disable | `plugins_list` 中该插件 `enabled=false` | 状态生效 |
| disable | 顶层工具数 **恰好** 减少 `plugin.tools.length` | 「它只贡献自己声明的那些」，不多不少 |
| disable | `ui_contributions` 的 `contributions[]` **无任何** `pluginId == id`，且总数 **恰好** 减少 `plugin.ui.length`；`slots[]` 同理 | 整面 + 片段一起消失 |
| disable | `plugin_rpc(id, '__matrix_probe__')` 报错且 message 含 `disabled` | 无越权旁路 |
| disable | 旧 IPC 垫片（`file_tree`/`preview_load`/`skills_list`/`mcp_list`）也返回 disabled | 宿主业务抽取后遗留的兼容面一起关 |
| enable | 工具数恢复 == 起始值；UI/slot 数恢复 == 起始值；RPC 不再 disabled | 可逆、无残留 |
| 全局 | `plugins.count >= 17`、`tools.count > 0`、`ui.payload_shape` 两数组、`pluginId` 全为非空字符串 | 契约形状（冻结 payload） |
| 第 2 次启动 | `config.json.plugins[id]=false` → 启动后即 disabled + 工具/UI 收缩 + 垫片 disabled | **持久化状态在启动时被尊重**（"restart 半场"） |

判据：Python 汇总 checks + per-plugin rows → `ok = 无失败 check 且所有 plugin row ok`；`--json` 输出 `{ok, dev_url, plugins, checks, failures}`；退出码 `0/1/2`（2=环境/构建问题，与断言失败区分）；产物 `.review/e2e/matrix/run/{report-matrix.json,report-persist.json,app-*.log}`。基线：**32/32 插件 · 342/342 checks**（2026-10-29 记录）。

### 5.4 它证明了什么 / 没证明什么（对「替换」的差距）

| 证明了 | 没证明 |
| --- | --- |
| 每个包贡献的**集合**是封闭且可移除的（运行时） | 包可以被**从构建里删掉**：没有「缺席构建」，也没有 `cargo build` 变体 |
| 禁用后 UI 整体与片段同时消失（`pluginId` 归属正确） | 组件闭包：matrix 只数 `ui.length`，**不检查 component id 能否被注册表解析**；缺组件只变成诊断卡 |
| 持久化 disabled 在启动时生效 | 替换：没有「用包 A' 替换 A，断言只有 A' 的贡献出现」的用例 |
| 宿主遗留垫片随包关闭 | 包间依赖/服务缺口：`provides/requires` 不与构建绑定 |
| 真实 IPC、真实二进制、真实 webview | 无 GUI 场景下的**便宜**判据（无 `ZEOKEEL_SELFTEST=registry-snapshot` 的矩阵化变体；selftest 只覆盖少量同步命令） |
| （成本参考）2 次启动 × 32 插件 ≈ 分钟级 | 32 个「缺席变体」×10 分钟构建 = 数小时；**验证经济学没解** |

**结论（写给新项目）**：matrix 的**句式**要保留并升级——把「disable 后恰好少掉它声明的东西」换成「**omit 后恰好少掉它声明的导出面**」，把「`ui.length` 计数」换成「**registry snapshot 集合 diff**」，把「持久化 config」换成「**package lock 里没有它 + 二进制里没有它的符号**」。详见 §12.3。

---

## 6. CI 与证据政策

### 6.1 四个 job（`.github/workflows/ci.yml`）

| job | 内容 | 阻塞 | 备注 |
| --- | --- | --- | --- |
| `rust` | 装 webkit2gtk-4.1 等 → `gen-module-manifests.py --check` → `gen-plugin-graph.py --check` → `ui-slot-report.py`（**没有 `--check`**）→ `cargo check -p zeokeel`（`RUSTFLAGS=-D warnings`）→ `cargo test --workspace --locked` → `cargo build` → 上传 debug 二进制 | 是 | 单次构建给 e2e 复用（避免二次 10 分钟） |
| `server` | 独立 workspace：`cargo check -p zeokeel-server --locked` + `cargo test --workspace --locked` | 是 | 不并入根 workspace |
| `frontend` | `pnpm install --frozen-lockfile` → `pnpm build` → `pnpm test:unit` → 显式验证 Chromium 存在 → `pnpm test:e2e` → **输出含 `N skipped` 即失败** | 是 | 反「假绿」 |
| `e2e` | 下载二进制 → `plugin-matrix.sh --no-build` | **advisory**（`continue-on-error: true`） | 注释与 `ci.md` 都承认：**从未在 GitHub runner 上跑过**（仓库长期无 remote），要求「runner 连续 3 次绿」才转正 |

### 6.2 证据政策（`docs/50-testing/strategy.md`，值得整体复用）

- **L0–L3 分级**：L0 模型自评（永不算数）/ L1 编译单测 / L2 真应用真 IPC 截图 / L3 独立验证者或干净环境复跑。
- **no judge tampering**：不许改断言、加 `#[ignore]`、`--skip`、mock 掉真检查来变绿；若任务就是要改判据，先记录基线并获用户确认。
- **skipped ≠ passed**；Playwright 无 Chromium 会 skip，CI 明确把 skip 变失败。
- **失败最多重试 3 次**并记录 flake；仍失败就诚实报失败，不扩大改动掩盖。
- 证据归档 `.review/evidence/<YYYYMMDD>/<task>/`（gitignored），截图/日志脱敏。
- 计数一律由 `./scripts/doc-metrics.sh` 现场生成（stdout，**不写文件**），文档只引用命令不手抄数字。

### 6.3 本地等价物

`./scripts/ci-local.sh yaml|rust|frontend|e2e|all` 复刻同一批命令；`yaml` 子命令只校验 workflow 结构。`docs/60-operations/ci.md` 有「Not verified (state honestly)」清单（GitHub 全流程、act 回放、24.04 真实包名、WebKit 稳定性、签名/公证、updater pubkey 仍是占位）——**这种诚实清单本身就是可复用资产**。

---

## 7. 明确的未完成项（target-vs-current，问题 5）

`docs/00-overview/target-vs-current.md` 是唯一权威表（文档自称），摘录 + 我的实测校正：

| Workstream | 文档状态 | 实测（`[工作区]`） | 真实差距 |
| --- | --- | --- | --- |
| Platform/plugin split | ⏳ `src-tauri/src/{agent,data,security,features}` 仍是业务大头 | 未变 | **最大硬缺口**；且抽取会留下 legacy IPC 垫片（matrix 的 `legacyMap` 就是它的化石） |
| Web bootstrap | ⏳ App.tsx 直接 import Shell | ✅ 已瘦身，Root 贡献已生效 | 文档滞后 |
| Generated plugin graph | ✅ Rust 半 / ⏳ UI 半 | ✅ 两半都有（`modules.generated.ts`） | 文档滞后 |
| UI inside module packages | ⏳ 7 个 UI module 在 `client/web/src/modules` | ✅ 已搬到 `plugin-core-ui/ui/`（本迁移未提交） | 文档滞后；且**目标布局里的 `rust/` 子目录未实现**（现为 crate 根 `src/`） |
| Equal permissions | ✅ 模型；声明清理可选 | 未变 | `permissions` 是全局门，不是分层——**继续保持** |
| Data slots | ⏳ 38/41 visual hosted，3 数据 slot 待服务 | 未变 | 数据 slot（theme/i18n/keybinding）没有 host 契约 |
| Server generated compose | ✅ | ✅ + `compose.rs` 双向校验 | 已成熟 |
| Server routes/modules | ✅ 13 / 266 | ✅ | — |
| Server admin UI | ⏳ P3 out of scope | 未变 | 不做 |
| （文档未列，实测新增）**组件闭包** | 无此条目 | **12 个声明了 UI 的插件包没有 `ui/`；10 个 component id 无人注册** | 新项目必须列为 blocking |
| （文档未列）**一包多清单** | 文档说「module.toml 是构建索引」 | 52 = 32+7+13，`plugin-core-ui` 独占 8 个 | 「一包一边界」未达成 |

> 另有 `.review/evidence/20261030/auditor/*.txt` 的机器审计（docs-stale 命中 20+ 处旧路径、计数 1025 vs 1026、37 vs 38 crate、10 个未注册组件、12 个无 ui 的 UI 声明包）。这是**先例留给你的负资产清单**，新项目不要重犯。

---

## 8. 返工点与教训（问题 6，≥5 条，全部带证据）

| # | 返工点 | 证据 | 根因 | 新设计的避免方式 |
| --- | --- | --- | --- | --- |
| 1 | **动态插件 C ABI / 安装器整套建成后删除**（`.so` 加载、`plugin-abi`、14 个 IPC、前端安装器 UI、10 条 Playwright） | `CHANGELOG.md:256`（Added：动态插件 C ABI v2）→ `:265-267`（Changed：插件静态化，2026-10-29 用户决策，全部删除）；`:152-153` 同文重复 | 把「包」误当成「运行时加载单元」；信任模型没先定 | **包 = 构建输入，替换 = 用不同包集重编**；永不引入 installer/ABI/market 作为「扩展性」答案；把「无运行时加载」写成架构决策并加守卫测试（禁止 `libloading`/`eval`/`.so` 出现在依赖里） |
| 2 | **UI 目录搬了 4 次**：`client/src/ui/components` → `client/src/modules/<id>`（2026-10-29）→ `client/modules/<group>/<crate>/ui/<id>`（2026-10-30 工作区）→ 目标 `client/modules/<group>/<id>/{rust,ui}` | `CHANGELOG.md:71`（前端模块化）；`:183`（删除 TS 内核与 14 modules）；`git status` 的 `RM client/web/src/modules/... -> client/modules/features/plugin-core-ui/ui/...`；`docs/00-overview/target-vs-current.md:40` | 包边界没和 UI 所有权一起定；先做功能后补归属 | **第一天就把「包目录布局」写成契约**（`<pkg>/{package.toml,rust/,ui/,tests/}`），生成器只认这一个布局；加「旧路径存在即失败」守卫（先例已有这个技巧：`gen-module-manifests.py:98`） |
| 3 | **`import.meta.glob` 自动发现 → 生成器静态 import** | `CHANGELOG.md:75`（「用 `import.meta.glob` 自动发现，删除 6 个中心注册文件」）→ `client/web/src/ui/registry/modules.ts` 现在写「generated static composition, **no discovery glob**」；`modules.generated.ts` 注释「Static imports, no `import.meta.glob`」 | glob 无法保证**确定性顺序**、无法对 `meta/register` 导出做 type-check、无法做「哪些包参与构建」的显式集合 | 直接做**生成器静态组合**；顺序策略是数据；生成文件必须 type-check（缺 `meta`/`register` 编译失败） |
| 4 | **给内核 quota 打补丁 → 两次重写 → 最终删除**（root `max_effects=64` 写死 → 按插件数缩放 `64+n*4` → 内核 0.5.0 语义变更后删除）；另加 `LifecycleBusy` 有界重试、`service_available` 泵送、`KERNEL_VERSION` 取错源 | `CHANGELOG.md:104-110`（quota 缩放 + 回退测试）、`:209-213`（LifecycleBusy 重试）、`:219`（KERNEL_VERSION 从 `CARGO_PKG_VERSION` 误取）、`:187-195,204-223`（内核 0.3→0.4.1→0.6.0） | 产品侧替内核内部实现兜底；内核 API 面没有兼容矩阵 | 内核是**外部版本化依赖**：维护「API 使用矩阵 + 接缝契约测试」（先例后来做了 `KERNEL-API-USAGE.md`）；**不 workaround**，向上游提 issue；版本号从 lock 读，不从 `CARGO_PKG_VERSION` |
| 5 | **`read_only` 由 risk 推导 → 计划模式被写工具绕过；DSH 工具写到进程 cwd（沙箱越界）** | `CHANGELOG.md:33-40`：`str_replace_editor` 声明 `Low` 被判只读，E2E 复现文件写到 cwd 而非工作区；`WorkingDirectoryService` 兜底 `current_dir()` | 把**安全相关属性**从粗粒度枚举「推断」出来，而不是声明 | 副作用**显式声明**（`effects = ["fs.read","fs.write",...]`）或由同源代码生成；安全门不吃「方便推断」；用真沙箱回归测试锁死；这是新项目包清单里 `permissions/effects` 段的设计依据 |
| 6 | **文档里的数字/路径反复过期**（多次专门的「fix stale counts」commit；auditor 命中 20+ 处） | `git log`：`bc5005f docs: fix stale test/plugin counts in README and CHANGELOG`、`42dd19b docs: fix stale Playwright counts...`、`57a91c3 ... refresh stale docs`、7 次以上的 `docs: refresh GAP §0.1 baseline`（30/69/39 → 31/72/40 → 32/73/41）；`.review/evidence/20261030/auditor/08-docs-stale.txt` | 数字靠手抄；文档 = 某一时刻快照，没有「文档 ↔ 代码」判据 | 数字**只生成、不手写**：`doc-metrics.sh --json` + 文档里用 marker 块注入（复用 §4 的 splice 技巧）+ CI `--check`；文档头部写「快照时间 + HEAD + dirty 数」（先例已有格式，直接抄） |
| 7 | **`ui_contributions()` 刷新把贡献子树 remount**（右面板 tab 重置、降级卡闪断） | `CHANGELOG.md:172-174`：改为「内容寻址缓存」后 Playwright 101/101 恢复 | 贡献的**身份**不稳定（每次 refresh 新对象 → React 重挂） | 把贡献 identity 写进契约（`packageId + contributionId + packageVersion/hash` 作 key）；加「toggle 无关包不得 remount 已渲染贡献」的测试 |
| 8 | **服务端 compose 手写 → 生成，迁移发生在所有模块存在之后** | `[HEAD]` 的 `server/bin/zeokeel-server/src/linked_modules.rs` 是手写 13 行 `#[cfg(feature=...)] use ... as _;`（且注释指向**不存在**的 `contracts/codegen/gen_modules.py`）；`[工作区]` 才改为 `include!` 生成文件 + `tests/compose.rs` 判据（两者都是未跟踪新文件）；`target-vs-current.md` 在 2026-10-30 才把「Server generated compose」标 ✅ | 组合点先手写、后补生成器：中间所有提交都在制造漂移；注释指向不存在的工具 = 没人能跑 | 生成器**先于第一个包**存在（哪怕 30 行 stub）；「删掉手写清单 + 生成物↔运行期双向测试」作为验收条件 |
| 9 | **计数判据自引用**：`EXPECTED_BUILTINS` 显式 26 项清单 ↔ 后来自动发现 | `CHANGELOG.md:177`「内置插件计数断言改回显式 26 项清单，避免判据自引用」；而 matrix/文档又说「脚本自动发现，不要手写计数」 | 判据必须独立于被测物；「自动发现」又会让「漏一个包」不可见 | 双层判据：**结构不变量**（每个包清单恰好注册一次、导出面闭包）替代魔数；另加**独立输入**（git 跟踪的包集 lock）diff |
| 10 | **GUI e2e 长期 advisory**（从未在 GitHub runner 跑过） | `.github/workflows/ci.yml` e2e job `continue-on-error: true` + 注释；`docs/60-operations/ci.md`「no GitHub remote」 | 昂贵 GUI harness 先于便宜判据；本地绿 ≠ 可阻塞 | 阻塞判据必须是**无 GUI** 的（headless snapshot + 变体构建）；GUI 矩阵保留为 advisory 但**有转正 SLA（3 次绿）**，先例已写了转正条件，抄它 |
| 11 | **生成器 `--check` 语义不统一 / 缺失** | `ui-slot-report.py`：`--check` 只查 missing host 且总写文件；`export-contracts.sh` 无 `--check`；CI 调用 `ui-slot-report.py` 无 `--check` | 每个脚本各写一套 CLI/写入逻辑 | 统一 runner：`codegen --check`（一次跑全部生成器）+ 生成物登记表；禁止「check 时也写文件」 |
| 12 | **legacy IPC 垫片 + 硬编码 plugin→垫片映射** | `scripts/plugin-matrix.sh:251 legacyMap()`（`tool-fs`→`file_tree`/`preview_load`，`plugin-skills`→`skills_list`，`plugin-mcp`→`mcp_list`）；`docs/20-client/ipc.md`「keep existing command names, add thin wrappers」 | 宿主业务抽取后保留兼容命令，归属没有建模 | 新项目：**一个 IPC 命令要么属于平台（白名单），要么由某个包声明**；matrix 泛化为「命令归属闭包」断言，删掉手写 map |
| 13 | **「免手写注册表」重构顺手丢弃了「可裁剪构建」** | `[HEAD]` `client/src-tauri/Cargo.toml:130-167`：32 个 per-plugin optional dep + `plugins-default` + 32 feature；`[工作区]` 改为单一 `plugin-bundle` 依赖、feature 全删 | 用「单一聚合」解决手写漂移，却没意识到 per-package feature 正是子集/替换构建的开关 | 生成器**同时**产出注册表与 feature 开关（两者都从同一包集派生，漂移仅一处）；把「能否构建最小集」写成验收命令（`cargo build --no-default-features --features minimal`），第一天就进 CI |

---

## 9. 清单 A：直接复用（按资产类别，全部给路径）

### 9.1 契约与数据格式

| 资产 | 复用方式 | 路径 |
| --- | --- | --- |
| `PluginManifest` / `Plugin` trait 形状 | 作为「包运行期接口」的基线；新项目加字段而不是重造 | `client/crates/plugin/src/contract/manifest.rs:284`；`.../runtime/registry.rs:29` |
| `ToolDecl` / `CapabilityDecl` / `CommandDecl` / `PromptSection` | 原样（`'static` const + Serialize） | `manifest.rs:91/146/269/255` |
| `UiContribution` / `UiSlotContribution` / `UiKind` | **原样搬**（含 `props` 为不透明 JSON 字符串这条硬规则） | `manifest.rs:159-254` |
| 41 个规范 slot 名与 kind | 原样（`single/list/keyed` 语义 + 未知名报错） | `client/web/src/ui/host/slots.ts`；`docs/UI-SLOTS.md` |
| 冻结 UI payload：`{contributions, slots}` + 每项 `pluginId`（camelCase） | 原样；这是 e2e 判据的基础 | `client/src-tauri/src/ipc/rpc.rs`（`ui_contributions`）；`shared/contracts/ui-contributions.json` |
| Root 即贡献 + `required` = 启动失败策略 + 可见 fallback 两态 | 原样（`Root` 变体与 `required()` 均为 `[工作区]` 新增） | `client/web/src/App.tsx`；`manifest.rs:377`；`client/modules/features/plugin-core-ui/src/lib.rs` |
| 用户数据格式硬约束（`config.json`/`workspaces.json`/`mcp.json`/conversations JSONL；新增字段一律 `#[serde(default)]`） | 原样（迁移已装用户的唯一办法） | `AGENTS.md`「用户数据兼容」；`docs/20-client/data-and-storage.md` |
| 权限模型：所有插件同权限；`permissions` = 与全局用户模式求交的粗声明；无 per-plugin 授权 | 原样（这是先例最正确的架构决策之一） | `docs/10-architecture/permissions-and-trust.md`；`client/src-tauri/src/data/state.rs::permission_set_for` |
| `ToolDecl::read_only()` 的保守推断规则 | 复用时**补显式 effects**（见 §10-8），规则可继续当兜底 | `manifest.rs:314` |
| `truncate_utf8`（CJK 截断不 panic） | 原样（真实踩坑产物） | `client/crates/plugin/src/lib.rs` |
| 服务器 `Module`/`ModuleManifest`/`ModuleCtx`/`inventory` 契约 | 原样；`tier`=启动策略、路由声明为契约、模块零 crate 依赖 | `server/crates/zeokeel-kernel/src/{module,ctx,host}.rs`；`docs/30-server/module-contract.md` |

### 9.2 生成器与构建机制

| 资产 | 复用方式 | 路径 |
| --- | --- | --- |
| 目录+清单发现、无中心列表 | 直接演进为包解析器 | `gen-plugin-graph.py:123` |
| 确定性排序（pin + topo），顺序语义显式 | 直接搬；建议把策略从脚本移到包清单数据 | `gen-plugin-graph.py:67-72,167` |
| `write_or_check` 精确字符串 + 签入生成物 | 直接搬 | `:422` |
| marker 块拼接（写入手写文件的一部分） | 直接搬（docs/Cargo/README/JSON 全用它） | `gen-server-compose.py:56,323` |
| 生成清单 ↔ 运行期真相双向交叉测试 | **最重要的判据模式**，直接搬 | `server/bin/zeokeel-server/tests/compose.rs` |
| 生成图行为测试（注册一次/唯一/顺序） | 直接搬 | `client/crates/plugin-bundle/tests/bundle.rs` |
| `[package.metadata.zeokeel]` 作为包自述 | 升级为完整 `package.toml` 的 `[package]` 段 | `server/modules/*/Cargo.toml` |
| 生成物目录布局（bundle + 前端 registry） | 保留「一个聚合 crate + 一个前端 registry」的形态 | `client/crates/plugin-bundle`；`client/web/src/ui/registry/modules.generated.ts` |
| 反向守卫：旧路径存在即失败 | 直接搬（迁移期防回退） | `gen-module-manifests.py:28,98` |
| **per-package Cargo feature + 命名集合**（`[HEAD]` 形态） | **必须重新引入**，但由生成器产出（不要手写）：`plugins-default` / `plugin-<id>` / 变体集合 | `git show HEAD:client/src-tauri/Cargo.toml`（`[features]` 段，130-167 行）；`git show HEAD:client/src-tauri/src/features/plugins.rs` |

### 9.3 测试方法论与证据

| 资产 | 复用方式 | 路径 |
| --- | --- | --- |
| matrix 的「恰好少掉它声明的」句式 + 双 boot（运行时切换 / 持久化启动） | 保留句式，升级为「构建缺席 + 导出面 diff」 | `scripts/plugin-matrix.sh:280-377` |
| matrix 的 harness（Xvfb/dbus/一次性 HOME/假模型/静态页/`file_write` 报告） | 整体搬（尤其「无 Playwright、经真实 IPC 写报告」） | `:380-470` |
| `--json`/`--only`/`--keep`/`--no-build` + 退出码 0/1/2 + artifacts 路径 | 直接搬 | 脚本头部 usage |
| `ZEOKEEL_SELFTEST=registry-snapshot|invoke:` 无 webview 快照 | **升级为一等公民**（新项目阻塞判据的基础） | `client/src-tauri/src/app/selftest.rs`；`docs/50-testing/e2e.md` |
| 证据政策 L0–L3 / no judge tampering / skipped≠passed / 3 次重试 / 证据归档路径 | 整体成为新项目 `docs/50-testing/strategy.md` | `docs/50-testing/strategy.md` |
| 反假绿：CI 验 Chromium 存在 + `N skipped` 即失败 | 直接搬 | `.github/workflows/ci.yml` frontend job |
| `doc-metrics.sh`（stdout only，口径写清） | 直接搬；再加文档 marker 注入 | `scripts/doc-metrics.sh` |
| 「Not verified (state honestly)」清单 | 直接搬为文化 | `docs/60-operations/ci.md` §Not verified |
| 每 crate 测试扫描（fails on zero tests）+ 单模块测试 | 直接搬，升级为「每包必有自己的替换测试」 | `scripts/test-all-modules.sh`；`scripts/module-test.sh` |
| 单模块 Playwright 映射（`module.json#e2e` → `module-test.sh` 跑对应 spec） | 直接搬 | `client/modules/**/ui/*/module.json` |

### 9.4 政策/流程

| 资产 | 路径 |
| --- | --- |
| 0-warning 策略（`-D warnings` 只作用于本 workspace） | `.github/workflows/ci.yml`；`AGENTS.md` |
| `#![forbid(unsafe_code)]`（插件内核） | `client/crates/plugin/src/lib.rs` |
| 源码即信任（构建输入 review + 依赖锁定 + 可复现构建），无沙箱承诺 | `docs/10-architecture/decisions.md` D3 |
| 平台不是插件（平台无特权） | `docs/10-architecture/decisions.md` D1 |
| 生成物不手改 + 文档维护清单 | `docs/70-reference/documentation-maintenance.md` |

---

## 10. 清单 B：必须重做（每条给「为什么今天不行」）

| # | 重做项 | 今天的事实 | 新设计要求 |
| --- | --- | --- | --- |
| 1 | **包清单（package manifest）v2** | module.toml 是派生索引：无 version、无 deps、无 exports、无 provides/requires、无 effects、无 hash、无 compat；三类 target 字段分裂；一包多清单（32+7+13） | 一包一清单、authored（或从单一源派生）、含 §12.1 全字段；client/server 同 schema 不同 `target`；UI 子模块作为**清单内数组**而非独立清单 |
| 2 | **单一真源方向** | 4 处拷贝：Cargo.toml / module.json / Rust const / 生成 module.toml | 选**一个方向**：`package.toml`（authored）→ 生成 `Cargo.toml` 片段 + Rust `const`（或 `build.rs` 注入）+ TS registry + docs；**禁止**再手写第二处；迁移用一次性 codemod（52 包） |
| 3 | **构建图：子集 + 变体** | `[工作区]` 单聚合 bundle、无 per-package feature、只能全量构建；`[HEAD]` 有 per-package feature 但注册表手写、不能表达变体集合 | 生成器同时产出：① `packages.lock`（集合+版本+hash）；② 聚合 crate 的 `PLUGIN_MODULE_IDS`；③ **给 `src-tauri` 注入 per-package optional dep + `[features]` + 命名集合（`full`/`minimal`/`<variant>`）**。CI 至少 3 个变体（full / minimal / 替换） |
| 4 | **组件闭包校验** | 声明 22 / 注册 16 / 测试 16，三份互漂；10 个 id 无人实现；`missingComponents` 只是运行期诊断 | codegen 产出 `component-closure.json`（provider=包，consumer=贡献），CI 断言：每个 declared id 恰有 1 个 provider；每个 registry id 有声明；**「data-only 贡献」必须显式类型化**（`kind=data` + 必需的 service host 测试），不能靠注释豁免 |
| 5 | **替换判据（build-variant matrix）** | 只有 runtime toggle matrix | 对每包（或分层抽样 + 全部关键包）构建「omit 变体」：编译通过、registry snapshot == baseline 减去该包导出面、无悬空 component、无未满足 provides/requires、headless 启动 OK、artifact manifest 显示包已移除；再加「swap 变体」（A→A' 同接口，断言只有 A' 的贡献出现） |
| 6 | **槽的 provider/consumer 建模** | slot 名有规范，但「谁提供、谁消费、缺了怎么办」没有模型；feature 插件声明 page 但无人校验可达性 | 包清单声明 `provides.slots` / `consumes.slots`；codegen 校验 consumers 有 provider 或显式 optional；缺 provider 在**构建期**失败 |
| 7 | **RPC 方法闭包** | `UiContribution.methods` 声明了，但没有「声明 ⊆ 实现」校验（对比 tool 名 ⊆ manifest 是有的） | 生成方法表 + 测试：每个声明的方法有实现、每个实现的（公开）方法有声明；`plugin_rpc` 未知方法报错而非静默 |
| 8 | **effects/副作用显式化** | `read_only` 由 risk 推导导致计划模式绕过（§8-5） | 包清单每个工具声明 `effects`（fs.read/fs.write/shell/network/model/…），`read_only` 由 effects 推导（方向反过来），沙箱回归锁死 |
| 9 | **顺序语义数据化** | `PIN_FIRST`/`ORDER_AFTER` 硬编码在生成器 | 包清单 `order.before/after` 或贡献自带的 order 全量数据化；生成器只做拓扑求解 |
| 10 | **生成器统一 CLI** | `--check` 有/无/语义不同 | 一个 `codegen` 入口：`generate` / `--check`（只读、绝不写）；生成物登记表；CI 一行检查全部 |
| 11 | **便宜判据优先** | GUI matrix 是唯一「真」证据，advisory 且慢 | headless selftest（registry snapshot / scripted invokes / build manifest dump）作为**阻塞**判据；GUI 只做「渲染真实性」抽查 |
| 12 | **计数/文档判据** | 手抄数字反复过期 | 文档 marker 注入 + CI `--check`；魔数改为结构不变量（§8-9） |
| 13 | **legacy IPC 垫片** | 宿主业务抽取遗留 `file_tree`/`preview_load`/`skills_list`/`mcp_list` + 硬编码映射 | 命令归属建模（平台白名单 / 包声明），matrix 泛化 |
| 14 | **服务端 compose 与客户端包模型统一** | 两套 schema（module.toml vs `[package.metadata.zeokeel]`）、两套生成器 | 同一 `package.toml` schema + 同一解析器，target 区分；服务端也要导出面/闭包校验 |
| 15 | **「一包 = 一个目录边界」** | 1+N 清单；UI 树曾经在包外；目标 `rust/` 子目录未实现 | 固定布局 `<pkg>/{package.toml,rust/,ui/,tests/}`；「删除目录 = 删除包」必须是**可执行断言**（生成器 + 测试 + 一次真实删包演练） |

---

## 11. 最大隐性成本（排序，附量化）

1. **验证经济学（第一成本）**。一次客户端构建 ~10 分钟（`ci.yml` 注释），32 个包 = 32 个缺席变体 ≈ 5 小时，不可接受。所以「CI 可替换」的真正难点不是生成器，而是**把替换验证做便宜**：
   - 好消息：先例 `[HEAD]` 已经有 per-package feature（`plugins-default` / 32 个 `plugin-*`），说明「裁剪构建」在 Cargo 层是通的，只是没有生成器、没有变体 CI、没有快照判据；
   - 坏消息：`[工作区]` 的重构把它删了——所以新项目要把「可裁剪」当作**不可回退的验收项**（`cargo build --no-default-features --features minimal` 必须绿）；
   - 变体构建必须能复用依赖缓存（只重建聚合 crate + 链接）；
   - 判定必须能在**不启动 GUI**的情况下做（registry snapshot / build manifest diff）；
   - 需要设计「分层抽样 + 关键包全量」策略（比如：全量跑 5 个关键包 + 其余包按变更集）；
   - 否则最终还是会退化成「跑一次 full build 看没炸」，替换性只剩口号。
2. **四份拷贝的收敛成本**。52 个清单 × 4 处拷贝 = 208 个改动点用于迁移；之后每加字段 4 处。必须**先写 codemod + `--check`**，再迁移，否则迁移本身就制造新漂移（先例的 UI 目录 4 次搬迁就是证据）。
3. **组件闭包的历史债**。今天 10 个已声明未实现 id + 3 个未接的数据 slot；若新项目照搬「data-only 宽容条款」，替换包时 UI 会静默降级成诊断卡而不是构建失败。修这个的成本不在代码，在**契约收紧**（要逐个给 10 个 id 找属主或删掉声明）。
4. **顺序语义**。注册顺序影响 UI 顺序与 system-prompt 片段顺序（`plugin-bundle/tests/bundle.rs` 显式断言 skills→rules→memory）。包一旦可换序/可替换，顺序必须数据化，否则每次替换改生成器代码 = 把配置问题变成代码问题。
5. **GUI e2e 的单次成本**。每次矩阵 2 次应用启动 + 90s 超时 + 重编（端口冲突时）；在 CI 上还是 advisory。它适合当「真实渲染抽查」，不适合当替换性主判据。
6. **文档即快照**。先例为此付出了 10+ 次专门 commit；不建「文档数字生成 + CI 校验」，新项目会在规模更大时重复付出。
7. **内核（zeokeel-cordis）的外部性**。0.6.0 已把 quota/LifecycleBusy/service_available 的语义改过若干次；产品侧每次都要适配。包模型不能依赖内核内部结构（fiber quota 之类），否则「包数变化 → 内核配额 → 矩阵 flake」会重演（`CHANGELOG.md:104`）。

---

## 12. 给 ZeoKeel 的落地骨架（可直接抄/改）

### 12.1 `package.toml` v2 字段草案（一包一清单）

```toml
schema = 2
[package]
id = "tool-fs"                     # 全局唯一
version = "0.1.0"                  # 包自身版本（不是 Cargo workspace 版本）
kind = "tool"                      # tool|capability|loader|feature|ui|server
target = "client"                  # client|server
title = "文件工具"
description = "..."

[build]
crate = "tool-fs"                  # Rust package 名
rust = "rust"                      # 目录（布局契约）
ui = "ui"                          # 可选；缺失 = 无 UI
tests = "tests"
features = []                      # 编译特性开关（用于变体）

[dependencies]                     # 包级依赖（解析器要能拓扑排序）
packages = []                      # 包间只允许声明到「导出面」，不允许 lib 互相依赖（客户端通常为空）
services = ["workspace", "approval"]# 需要平台提供的 host services

[provides]
components = []                    # 本包注册的 component id
slots = []                         # 本包提供的 slot（如 shell）
tools = ["read_file", "write_file"]
commands = []
services = []                      # 对外提供的服务
rpc = []                           # plugin_rpc 方法

[consumes]
slots = []                         # 依赖他人提供的 slot（缺 provider 构建期失败）
services = ["workspace"]

[effects]
tools = { read_file = ["fs.read"], write_file = ["fs.write"] }

[order]
after = []                         # 替代生成器里的 ORDER_AFTER
before = []
required = false                   # 启动失败策略（不是权限分层）

[ui]                               # UI 子模块作为数组（不再一模块一清单）
[[ui.modules]]                     # 一个包可拥有 0..N 个 UI 模块
id = "core-shell"
entry = "ui/core-shell/module.tsx"
components = ["core.sidebar", "core.commands"]
e2e = ["v2-shell.spec.ts"]

[compat]
lock = "packages.lock"             # 包集锁：name+version+sha256
```

> 迁移策略：先把现有 `module.toml` + `module.json` + `Cargo.toml` 合并生成 v2（一次性脚本），**dual-read 一段时间**，再删旧格式；同时给 `codegen --check` 加「旧格式存在即失败」守卫（抄 `gen-module-manifests.py:98`）。

### 12.2 生成器拆分（3 生成 + 1 校验，替代今天 5 个各写各的）

| 阶段 | 职责 | 输出 | 判据 |
| --- | --- | --- | --- |
| `analyze` | 发现包目录、读 `package.toml`、校验 schema/布局/唯一性 | 内存图（包集 + 边） | 字段缺失、未知键、布局不符、重复 id → 失败 |
| `resolve` | 拓扑排序、求解 `provides/requires`、闭包（组件/槽/服务/RPC） | `packages.lock`（集合+版本+hash）+ 诊断 | 缺 provider、环、版本冲突 → 失败 |
| `emit` | 生成：聚合 crate（Rust 注册）+ 前端 registry（静态 import）+ Cargo features/变体 + docs 索引/契约 JSON | 全部签入生成物 | 确定性（同输入同输出） |
| `verify` | `--check` 只读比对 + 生成物行为测试 + 生成清单 ↔ 运行期真相双向交叉 | exit 0/1 | 任何漂移 → 失败；**禁止 check 时写文件** |

### 12.3 CI 替换矩阵（分层，把成本压到可接受）

| 层 | 判据 | 成本 | 何时跑 |
| --- | --- | --- | --- |
| L1（阻塞） | `codegen --check`；生成清单 ↔ 运行期 registry 双向交叉；组件/槽/服务闭包 | 秒级 | 每次 |
| L2（阻塞） | **omit 变体**：分层（全部关键包 + 变更集触达的包）无 GUI 构建 + `--selftest registry-snapshot` diff（baseline 减去该包导出面，且无悬空 id） | 每变体 ≈ 链接 + 秒级启动 | 每次 PR（缓存依赖） |
| L3（阻塞，抽样） | **swap 变体**：A→A'（同接口最小实现），断言只有 A' 的贡献出现 | 同 L2 | 每晚 / 发布前 |
| L4（advisory） | 现有 GUI 矩阵（真 webview 渲染、RPC、持久化启动） | 分钟级 | 每晚 + 发布前，3 次绿后转正（抄先例的转正条件） |

### 12.4 第一天就要写的守卫测试（可直接抄的先例判据）

1. 每个包目录恰好一个 `package.toml`，且布局符合契约（否则失败）。
2. 每个包的 `provides` 在生成物里恰好出现一次；`consumes` 都能被满足。
3. 每个 component id 恰有一个 provider；无未注册声明；无未声明注册。
4. 注册顺序 == 解析结果的确定性顺序（同输入两次生成 byte-identical）。
5. 生成清单 ↔ 运行期 truth 双向交叉（抄 `compose.rs`）。
6. 删掉某包目录 → `codegen` 失败（提示 lock 需更新），更新 lock 后构建通过且 snapshot 只少该包。
7. 「旧路径/旧格式存在即失败」守卫（抄 `gen-module-manifests.py:98`）。
8. 文档数字 marker 块 == `metrics --json` 输出。

---

## 附录 A：关键路径索引（按主题）

| 主题 | 路径 |
| --- | --- |
| 契约（Rust） | `client/crates/plugin/src/contract/{manifest,tool,error}.rs` |
| 运行期 | `client/crates/plugin/src/runtime/{registry,host,hooks,agent,events}.rs` |
| 服务缝 | `client/crates/plugin/src/services.rs` |
| 生成聚合 | `client/crates/plugin-bundle/{Cargo.toml,src/lib.rs,tests/bundle.rs}` |
| 前端贡献渲染 | `client/web/src/ui/host/**`、`client/web/src/ui/registry/**`、`client/web/src/App.tsx` |
| 前端模块 | `client/modules/features/plugin-core-ui/ui/<id>/{module.json,module.tsx,module.toml}` |
| 生成器 | `scripts/{gen-module-manifests,gen-plugin-graph,gen-server-compose,ui-slot-report}.py`、`scripts/export-contracts.sh` |
| 矩阵 | `scripts/plugin-matrix.sh`、`scripts/e2e-tauri.sh`、`scripts/live-tools-e2e.sh` |
| 指标 | `scripts/doc-metrics.sh`、`docs/70-reference/metrics-baseline.md` |
| CI | `.github/workflows/{ci,release}.yml`、`scripts/ci-local.sh` |
| 契约产物 | `shared/contracts/{ipc-commands,ui-contributions,ui-slots}.json` |
| 服务端 | `server/crates/zeokeel-kernel/src/**`、`server/bin/zeokeel-server/src/linked_modules*.rs`、`server/bin/zeokeel-server/tests/compose.rs` |
| 目标/差距 | `docs/00-overview/target-vs-current.md`、`docs/10-architecture/{decisions,module-packages,plugin-contract,ui-contribution-model,cordis-runtime,permissions-and-trust}.md` |
| 审计证据 | `.review/evidence/20261030/auditor/*.txt`、`.review/evidence/20261030/server-compose/` |
| 历史返工 | `CHANGELOG.md`（行号见 §8） |

## 附录 B：只读核验命令（在 `/data/ZeoKeel-Next` 下）

```bash
# 现状计数（不要手抄）
./scripts/doc-metrics.sh

# 三个生成器漂移检查（注意 ui-slot-report 的 --check 语义不纯）
python3 scripts/gen-module-manifests.py --check
python3 scripts/gen-plugin-graph.py --check
python3 scripts/gen-server-compose.py --check

# 真实 IPC 矩阵（需要 Xvfb/openbox/dbus）
./scripts/plugin-matrix.sh --no-build --json

# 服务端模块/路由
cd server && cargo run -p zeokeel-server -- --dump-modules

# 组件闭包实测（本报告 §3.5 的 22/16/16 与 10 个未注册 id）
python3 - <<'PY'
import json,re,pathlib
declared=set()
for p in pathlib.Path('client/modules').glob('*/*/src/lib.rs'):
    t=p.read_text()
    declared|=set(re.findall(r'component:\s*"([A-Za-z0-9_.-]+)"',t))
reg={}
for m in pathlib.Path('client/modules').glob('*/*/ui/*/module.json'):
    d=json.loads(m.read_text()); reg.update({c:d['id'] for c in d.get('components',[])})
print("declared:",len(declared),"registered:",len(reg))
print("missing impl:",sorted(declared-set(reg)))
PY
```

## 附录 C：本报告实际读过的文件（可复核）

`AGENTS.md`、`README.md`、`CHANGELOG.md`、`docs/{README,CI,E2E,MODULE-INDEX,UI-SLOTS}.md`、`docs/00-overview/{target-vs-current,repository-layout}.md`、`docs/10-architecture/{plugin-contract,module-packages,ui-contribution-model,cordis-runtime,decisions,permissions-and-trust,platform-and-plugins}.md`、`docs/20-client/{plugin-registry,ipc,web-renderer,layout}.md`、`docs/30-server/{module-contract,composition}.md`、`docs/40-build/build-system.md`、`docs/50-testing/{strategy,client-tests,e2e}.md`、`docs/60-operations/ci.md`、`docs/70-reference/{plugin-catalog,scripts,metrics-baseline}.md`、`client/crates/plugin/src/{lib.rs,contract/manifest.rs,runtime/registry.rs}`、`client/crates/plugin-bundle/{src/lib.rs,tests/bundle.rs}`、`client/modules/tools/tool-time/{Cargo.toml,src/lib.rs}`、`client/modules/features/plugin-core-ui/{module.toml,src/lib.rs,tests/contract.rs,ui/README.md,ui/*/module.{json,tsx,toml}}`、`client/web/src/{App.tsx,ui/registry/modules.{ts,generated.ts},ui/host/*}`、`client/src-tauri/src/{ipc/rpc.rs,ipc/feature_ipc.rs,app/selftest.rs}`、`server/bin/zeokeel-server/tests/compose.rs`、`scripts/{gen-module-manifests.py,gen-plugin-graph.py,gen-server-compose.py,ui-slot-report.py,export-contracts.sh,plugin-matrix.sh,doc-metrics.sh,ci-local.sh}`、`.github/workflows/ci.yml`、`shared/contracts/*.json`、`.review/evidence/20261030/{auditor/*.txt,server-compose/*}`、`git log/status/diff --stat`。
