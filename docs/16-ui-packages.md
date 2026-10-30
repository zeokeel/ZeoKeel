# 16 · 前端也是包：注册表、槽位、类型契约、构建期换包

> 依据 `research/frontend.md`（DSH 305 包裹 + 先例 React 渲染器 + crates.io 实测）。
> 结论先行：**前端包 = pnpm workspace 成员包（带 `zeokeel.client` 清单）+ 构建期生成的静态注册表；
> 换包 = 改 profile 后重新 compose + 构建；shell 只认识 `componentId` / `slotId`，不认识包名。**

## 1. 形态选型

| 维度 | pnpm workspace 成员包 | npm 独立包+版本 | Rust crate 内嵌 UI | 单文件 ESM 清单 |
| --- | --- | --- | --- | --- |
| 构建期可替换 | 强 | 强 | 弱 | 中 |
| 类型安全 | 强（project references） | 强（需 d.ts） | 弱 | 弱 |
| HMR | 好 | 差 | 中 | 差 |
| 单二进制内嵌 | 好（一次 build → frontendDist） | 好 | 好 | 好 |
| tree-shaking / React 单实例 | **最好** | 中（易重复 React） | 好 | 差 |
| CI 复杂度 | 低 | 中（发布流程） | 中 | 低但脆 |
| 供应链可控 | 强（lockfile） | 中 | 强 | 弱 |

**采用**：pnpm workspace 成员包为主形态；OEM 定制 UI 由我方开发并入库（D61），**不做第三方预构建包**。

## 2. 目录形态

沿用 01/05 的"一包一目录"，UI facet 在包内 `web/`：

```text
packages/
  ui/shell-kit/           # UI 包 zeokeel/ui-shell-kit：壳原语 + SLOT_DEFS/props 常量（角色 ui.shell-kit，singleton；契约 crate 仍是 zk-contract-ui）
    web/package.json      # @zeokeel/ui-shell-kit（SLOT_DEFS/props 类型；角色 ui.shell-kit）
  ui/shell/               # 组装壳：只 import @zeokeel/ui-shell-kit + virtual:@zeokeel/registry
  server/admin-shell/     # 管理台壳（角色 admin.shell；web/ 为 @zeokeel/admin-shell）
  ui/chat/                # 业务 UI 包
  ui/settings/ ...
  client/transport-tauri/ # invoke() 实现（角色 ui.transport）
  client/transport-http/  # fetch + WS 实现（serve / CI）
  client/transport-mock/  # 无后端 mock（替代先例的 window.__TAURI_INTERNALS__ 嗅探）
```

根 `pnpm-workspace.yaml`（唯一 workspace root，glob 新增包免改根清单）：

```yaml
packages:
  - "packages/*/*/web"
  - "apps/*/web"            # desktop/web、admin/web
```

每个 UI facet 的 `package.json` 必须带清单：

```jsonc
{
  "name": "@zeokeel/ui-chat",
  "type": "module",
  "exports": {
    ".":            { "types": "./src/index.ts", "default": "./src/index.ts" },
    "./contribute": { "types": "./src/contribute.ts", "default": "./src/contribute.ts" }
  },
  "peerDependencies": { "react": "^19", "@zeokeel/ui-shell-kit": "workspace:^" },
  "zeokeel": {
    "client": {
      "platform": "web",
      "role": "chat",                                  // 与 zk.toml 的角色一致，composer 校验
      "baseline": ["react", "react-dom", "@zeokeel/ui-shell-kit"],  // 允许的外部 import（包边界）
      "components": ["chat.main", "chat.composer"],    // 本包提供的 component id
      "inject": ["session", "workspace"]               // 需要宿主提供的上下文服务
    }
  }
}
```

`baseline` 是**包边界护栏**：单一构建里没有物理边界，所以用「声明 + import 图检查」实现。任何
超出 baseline 的 import（例如直接 import 另一个 UI 包的内部组件）由 `scripts/lint-boundaries.mjs`
读 rollup metafile 拒绝。OEM 代码同样走本仓 review 与 baseline 校验。

## 3. 贡献 / 槽位模型

**真源只有一个：`zk.toml` 里的静态声明。** 运行期不允许产生新的贡献（否则构建期无法做闭包校验）。

```toml
# packages/ui-chat/zk.toml
[facets]
rust = { crate = "zk-ui-chat" }
ui   = { package = "@zeokeel/ui-chat" }

[[ui.contributions]]
slot         = "ui.chat"
component    = "chat.main"
order        = 10
props_schema = "schema/chat-main.props.json"
```

**槽位 id 规则**：客户端 slotId 带 `ui.` 前缀（管理台见下方 admin 注册表）；贡献角色名 = `ui.slot.<slotId 去掉 ui. 前缀>`（例 slotId `ui.chat` → 角色 `ui.slot.chat`；`ui.settings.section` → 角色 `ui.slot.settings.section`），与 `02` §4/`03` 示例一致。

**`SLOT_DEFS`（冻结；`zeokeel/ui-shell-kit` 包唯一声明，其它文档只引用）**

| slotId | 用途 | props（必需） |
| --- | --- | --- |
| `ui.login` | 登录页 | `channel` |
| `ui.onboarding` | 个人版引导（选 provider/填 key） | `providers` |
| `ui.first-run` | 官方企业版首配地址 | `defaultServerUrl?` |
| `ui.gate.update` | 强制更新门禁 | `manifest` |
| `ui.gate.account` | 账号禁用/撤销 | `reason` |
| `ui.gate.maintenance` | 维护页 | `notice?` |
| `ui.gate.channel` | 渠道/版本不匹配 | `expected` |
| `ui.shell` | 主布局 | `sections` |
| `ui.chat` | 对话区 | `conversationId` |
| `ui.composer` | 输入区 | `conversationId` |
| `ui.toolcall` | 工具调用卡 | `call` |
| `ui.approval` | 审批卡 | `request` |
| `ui.settings` | 设置页 | `section?` |
| `ui.settings.section` | 设置页分节（multi，多包贡献） | `section` |
| `ui.skills` | 技能页 | `items` |
| `ui.connectors` | 连接器页 | `items` |
| `ui.apps` | 应用页（企业版） | `items` |
| `ui.update` | 更新页 | `state` |
| `ui.diagnostics` | 诊断页 | `report` |

**管理台 slot 注册表（`admin.*`；唯一真源在本表，`08` 只引用）**

| slotId | 用途 | props（必需） |
| --- | --- | --- |
| `admin.nav` | 管理台导航项（multi） | `permission` |
| `admin.page.<page>` | 页面主体槽（multi；贡献角色 `admin.slot.page.<page>`；页面实现角色 `admin.page.<page>` 为 singleton） | `permission` |

管理台贡献角色名 = `admin.slot.<slotId 去掉 admin. 前缀>`（如 `admin.nav` → `admin.slot.nav`；`admin.page.users` → `admin.slot.page.users`）；
页面实现角色 = `admin.page.<page>`（singleton，选页面包），两者分工见 `08` §3。

composer 在构建期做四件事：

1. 把 `[[ui.contributions]]` 生成到 `shared/contracts/ui-contributions.json`，并生成 Rust `const
   CONTRIBUTIONS: &[UiContribution]`（声明 crate 只含该静态常量、无运行逻辑；`zk-ui-chat` 的 `install()` 只注册它）；
2. 校验 `component` ⊆ 所有被选中 UI 包的 `components`（闭包校验，缺一个就构建失败）；
3. 校验 `slot` 在客户端 `SLOT_DEFS` 或管理台 slot 注册表（本文件 §3，`admin.*`）里存在，且 `props_schema` 与 slot 的 props 类型兼容；
4. 生成 `registry.gen.ts`（**只有组件表**，不含 order/props——那些来自契约 JSON）。

TS 侧只声明"我提供哪些组件"：

```ts
// web/src/contribute.ts
export const components = {
  "chat.main": ChatMain,
  "chat.composer": ChatComposer,
} satisfies Record<string, React.ComponentType<SlotProps>>;
```

shell 不 import 业务组件：

```ts
// packages/ui-shell/web/src/App.tsx
import { registry, SLOT_DEFS } from "virtual:@zeokeel/registry";
import { contributions, subscribeContributions } from "@zeokeel/ui-shell-kit";

export function App() {
  const enabled = useContributions();          // 启动后由后端下发"哪些包激活了"
  return <Shell slots={SLOT_DEFS} items={enabled} />;
}
```

- 运行期只做**过滤**：后端在注册完成后下发"已激活的包 id 集合"，shell 用
  `contributions.filter(c => enabled.has(c.packageId))` 渲染。
- `virtual:@zeokeel/registry` 由 vite 插件解析到 `apps/desktop/web/src/generated/registry.gen.ts`
  （不放在独立 pnpm 包里，避免锁文件 importer 随 profile 变化）。
- **组件缺失或抛错时必须是显式 fallback**：渲染错误卡片（componentId + 包 id），错误进诊断面板；
  不允许静默空白。

### 3.1 四件事的真源：槽位 / 组件 / 贡献 / 运行期注册表

| | 真源 | 何时确定 | 用途 |
| --- | --- | --- | --- |
| **槽位定义** `SLOT_DEFS` | 包 `zeokeel/ui-shell-kit`（枚举 + props 类型；契约 crate 是 `zk-contract-ui`） | 编译期 | shell 知道有哪些位置、每个位置接受什么 props |
| **组件提供** `components` | 各 UI 包 `contribute.ts` | 构建期（生成 registry） | component id → 组件实现 |
| **贡献实例** `contributions` | 各包 `zk.toml` 的 `[[ui.contributions]]` | **构建期**（生成 const + JSON） | 谁放在哪个槽、顺序、props schema |
| **运行期注册表** | `registry.gen.ts`（构建期生成）+ 已激活包集合（后端下发） | 编译期 / 启动后 | **只做过滤**：未激活包的贡献不渲染 |

这样构建期才能做闭包校验；"随包启停"由运行期过滤实现，不需要动态产生贡献。

**闭包校验**（先例教训：声明 22 / 注册 16 / 测试 16 三份互漂，10 个 id 无人注册）：

| 检查 | 失败时 |
| --- | --- |
| 静态 `contributions[].component` ⊆ 所有被选中 UI 包的 `components` | 构建失败，列出缺失 id + 来源包 |
| 每个 component id 恰好一个提供者 | 构建失败，列出冲突 |
| `slot` 在 `ui-shell-kit` 的 `SLOT_DEFS` 里存在 | 构建失败（防止拼写错误静默丢 UI） |
| `props_schema` ⊆ slot 的 props 类型（必填字段齐备） | 构建失败 |
| 每个被选中的 UI 包至少贡献一个组件或槽位 | 警告（防止"加了包却没接进组合"） |

## 4. 换 UI 包：只有一条主通道

| 机制 | 定位 | 说明 |
| --- | --- | --- |
| **edition 基线 + profile/channel 覆盖** | **唯一换包通道** | 改 edition 基线或 profile 覆盖一行包 id → `zk compose` → 重新 build |
| ~~`pnpm.overrides` / `npm:` 别名~~ | **不使用** | 绕过 `zk.lock` 与生成物 diff 校验；OEM 需要定制就出源码包（D61） |
| ~~`vite resolve.alias`~~ | **不使用** | 同上；只在临时排障时手工改，不纳入 CI 流程 |

edition 基线片段（profile/channel 只覆盖 singleton；multi 增删用 `[packages.add]`/`[packages.remove]`，`24` §1.1）：

```toml
# editions/personal.toml [packages]
"ui.shell"                 = "zeokeel/ui-shell"
"ui.transport"             = "zeokeel/transport-tauri"
"brand.mark"              = "zeokeel/brand-official"     # OEM: 换品牌包
"theme.tokens"             = "zeokeel/ui-theme-default"

# multi 角色：数组顺序即渲染顺序；增删在 profile 用 add/remove
"ui.slot.settings.section" = ["zeokeel/ui-settings-general", "zeokeel/ui-settings-models"]
```

版本不进 profile：`zk.lock` 记录解析结果（含 npm integrity），CI 比对 `compose-report.json` 确保
"换包只换该换的"。OEM 定制包必须源码入库，走同一份 ESM 清单 + `baseline` + integrity
接入，仍是同一条通道。

## 5. Rust ↔ TS 类型契约

**槽位 props 必须有类型**（评审盲点 6）：`zeokeel/ui-shell-kit` 为每个槽位定义 props 类型（TS）；对应
serde DTO 与 `SlotId` 在契约 crate `zk-contract-ui`，经 ts-rs 导出。贡献声明里的 `props_schema` 必须兼容该类型；运行期用 JSON Schema 校验
后仍要经 `ComponentProps` 类型检查。**禁止 `ComponentType<any>`**——这是先例"UI 声明/注册/测试三份漂移"
的直接成因。

三类面分开处理，不要一套方案硬套：

| 面 | 方案 | 产物 |
| --- | --- | --- |
| DTO / 事件 / 贡献描述符（serde 结构） | **ts-rs**（`#[derive(TS)]` + `cargo test export_bindings`） | `shared/ts/*.ts`（`24` §3），CI `git diff --exit-code` |
| 工具参数 / 配置 schema（本来就要 JSON Schema） | **schemars** | `schema/*.json`，前端用 `json-schema-to-ts` 取类型，或运行时校验 |
| IPC 命令表 | 生成器 | `shared/contracts/ipc-commands.json` + 前端 `commands.gen.ts`（name→args/result 类型） |

不推荐 typeshare（类型映射覆盖面窄）、quicktype（多一层生成与调试成本）。
ts-rs 的坑：枚举 tagging 必须与 serde 一致（`#[serde(tag=..., content=...)]` 要同步 `#[ts(...)]`），
所以 DTO 定义放契约 crate、只在一处写标注。

## 6. 桌面壳：Tauri 2 + 同源 HTTP 副载体

| 维度 | Tauri 2 | wry 自建 | 内嵌 HTTP + 浏览器 | egui/slint |
| --- | --- | --- | --- | --- |
| 单二进制 | ✅ | ✅ | ❌（要起服务） | ✅ |
| 前端包化可行 | ✅ | ✅ | ✅ | ❌（无 npm 生态） |
| CI 无头 e2e | 受限（macOS 无 WebDriver） | 同 | ✅（Playwright 直连 HTTP） | 需图像对比 |
| 远程访问 | 需另做 | 需另做 | ✅ 天然 | ❌ |
| 体积 | 中 | 小 | 小 | 最小 |
| 签名/公证/更新 | ✅ 成熟 | 自己造 | 自己造 | 中 |

**决策**（ADR-006 + ADR-018）：`platform-tauri` 为主壳；同一份 `dist` 由 `host-webserver` 包
（axum）以本地 HTTP 提供，用于浏览器调试与 CI 无头 e2e。两者都实现 `zk-contract-platform` 的
`ShellHost` / `HttpHost`，**由 profile 选择**：

```toml
"platform.shell" = "zeokeel/platform-tauri"
"platform.http"  = "zeokeel/host-webserver"   # 不写就没有 serve 载体
```

`apps/desktop/Cargo.toml` 由 `zk compose` 生成（只依赖被选中的 platform 包），`main.rs` 固定为
`fn main() { zk_bootstrap::run() }`——**换壳只改 profile，不改应用代码**（评审 S7）。

前端通过 **Transport 抽象**切换：

```ts
// @zeokeel/ui-shell-kit
export interface Transport {
  call<T>(name: string, args: unknown): Promise<T>;
  subscribe(stream: string, cb: (ev: IpcEvent) => void): () => void;
}
```

三个实现各是一个包，profile 选一个：`transport-tauri`（invoke + event）、`transport-http`
（fetch + WebSocket）、`transport-mock`（无后端，固定回放，供 UI 单测与离线演示）。

**收益**：Playwright 跑 `serve` 载体即可覆盖真实 IPC，不受 WebDriver 平台限制；同一份代码也能
支持未来"浏览器访问本地 agent"。代价：IPC 契约必须传输无关（见 03 §6），不能直接用 Tauri 类型。

**serve 载体在 M1 就带最小安全**：只绑 `127.0.0.1`、一次性 token、校验 Origin、拒绝跨源写操作。
真正的远程访问（0.0.0.0 + TLS + 鉴权策略）是 M5 的独立议题，不能默认开启。

## 7. 构建与 CI

```bash
# 本地 / CI 同一套
zk compose --edition personal --profile default --channel official   # 生成 apps/desktop/web/src/generated/*、crates/zk-composed；contracts 由 zk api、ts 由 ts-rs
pnpm install --frozen-lockfile          # 依赖是"候选包并集"，锁文件与 profile 无关
pnpm -r typecheck                       # tsc 跨包
pnpm -r test:unit                       # vitest（组件 + 契约）
pnpm -C apps/desktop/web build          # vite build → apps/desktop/web/dist（profile 由生成期固定）
node scripts/verify-dist.mjs            # 产物校验（见下）
```

`verify-dist.mjs` 必做项：React 单实例（无重复打包）、无 `eval`/内联脚本（CSP）、
生成物哈希与 `compose-report.json` 一致、bundle 体积预算、sourcemap 关闭（release）。

Playwright：按 profile 分 project，跑的是**构建产物 + `serve` 载体**，不是 dev server：

```ts
projects: ["default", "mock"].map(p => ({
  name: p, use: { baseURL: `http://127.0.0.1:${portFor(p)}` },
}))
```

CI 矩阵（与 03 §6 合并）：profile `default | mock | minimal | hardened` × channel `official | oem-demo`
（compose → typecheck → build（变体用 `--offline`）→ verify-dist → vitest → Playwright 冒烟）。
`default` 用 `--frozen-lockfile`，变体在 CI 内重新解析（见 03 §5.5 / ADR-015）。

## 8. 风险与预埋

| 风险 | 缓解 | 守卫 |
| --- | --- | --- |
| WebKitGTK / WebView2 / WKWebView 行为差异（最大） | M1 起三平台 `cargo check` + 每平台冒烟；browserslist + `@supports` 降级；关键交互真机验收。**注意**：Linux 上的 Playwright `webkit` project 跑的是 WebKitGTK，不等于 WKWebView（研究结论），所以它只是近似，不能替代 macOS 真机 | 每平台 smoke job（M1 起）；发布前人工验收清单 |
| npm 供应链 / 离线构建 | lockfile 入库、`pnpm fetch` 缓存、私有 registry 可切换、`pnpm audit` | `supply-chain` job 阻断 |
| 换包后 id 覆盖不足 | composer 闭包校验 | 构建失败 |
| React 双实例 | 单次构建 + peerDependencies 约束 + verify-dist 检查 | 构建期失败 |
| CSP | 无内联脚本/样式（Tailwind 编译期）、`connect-src` 按载体生成 | verify-dist + 真机 console 断言 |
| HMR 与包边界 | 跨包 HMR 依赖 vite workspace 解析；包必须导出源码入口（不导出预构建产物） | dev 冒烟 |
| OEM 定制 UI 包 | 源码入库 + baseline 校验 + 同一 CI 门禁 | compose 期校验 |
