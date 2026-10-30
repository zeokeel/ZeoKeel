# 前端与桌面壳研究（ZeoKeel）

> 任务 task-5。目标：给出"前端组件也是包 + 任何包可在 CI 构建期被替换"的可执行结论。
> 证据来源：本机 DSH 安装（305 个 `@deepseek-ai/*` 包，版本 `0.2.1-rc.2`）、`/data/ZeoKeel-Next`（只读先例）、crates.io 与官方文档（2026-10-30 经 `proxychains -q` 查询）。

## 0. 结论先行

| # | 问题 | 结论 |
|---|---|---|
| 1 | 前端包形态 | **pnpm workspace 成员包 + 每包 `zeokeel.client` 清单 + 构建期生成的静态注册表**。不发布 npm（至少暂时），不用纯 ESM 清单当主形态，不把 UI 塞进 Rust crate 源码树。 |
| 2 | "CI 换包"怎么换 | **换 profile（角色→包映射）+ 跑一个 composer 生成注册表，再正常构建**。`vite resolve.alias` 只用于"同 specifier 换实现"的兼容层，不是主机制；`pnpm overrides`/`npm:` 别名用于锁文件级替换；**生成 registry 是主机制**。 |
| 3 | 贡献/槽位 | Rust 插件声明**贡献**（`slot + component id + order`），TS 包声明**提供**（`components` 列表 + `./contribute` 入口）。composer 交叉校验"需要的 id ⊆ 提供的 id"，不满足就构建失败。shell 只认识 id 与 slot 名，不认识包名。 |
| 4 | Rust↔TS 类型 | **ts-rs** 为主（DTO/事件/贡献描述符），**schemars** 仅用于工具参数等"本来就要 JSON Schema"的面。typeshare、quicktype 不推荐作为主链路。 |
| 5 | 桌面壳 | **Tauri 2（2.12.x）主壳 + 同一份 dist 的 `zeokeel serve`（axum/本地 HTTP）副载体**，前端走 transport 抽象层。理由：单二进制、生态、签名公证齐备；而 CI 用 HTTP 载体 + Playwright 免掉 WebDriver 的 macOS 缺口。egui/slint 直接排除（与 npm 包体系不兼容）。 |
| 6 | 最大风险 | **WebKitGTK（Linux）与 WebView2/WKWebView 的行为差异**（等价性无法只靠"前端跑了"证明）；其次 **npm 供应链与离线构建**，再次 **换包后的 id 覆盖率/React 单实例**。 |

一句话版本：*前端包 = pnpm workspace 里的 npm 包（每个包带一份清单文件）；构建期由一个 composer 按 profile 把"角色→包"解析成一张静态注册表，Rust 侧贡献按 component id 去表里查组件；壳用 Tauri 2，同一 dist 另有一条 `zeokeel serve` 本地 HTTP 通道供 CI 与远程访问。*

---

## 1. 前端"包"的形态选型

### 1.1 候选与对照

| 维度 | (a) pnpm workspace 成员包 | (b) npm 独立包 + 版本 | (c) Rust crate 内嵌 UI | (d) 单文件 ESM 清单 |
|---|---|---|---|---|
| 构建期可替换 | **强**：改依赖/改 profile 重装即可 | 强（版本号即替换） | 弱：换实现=换 crate + 重建 Cargo 依赖图 | 中：换清单条目即可，但无依赖解析 |
| 类型安全 | **强**：TS project references + tsc 跨包检查 | 强（需发包时带上 d.ts） | 弱：前端类型与 Rust 生命周期脱钩 | 弱：清单无类型，靠约定 |
| HMR / 开发体验 | **好**：Vite 直接读源码，跨包 HMR 天然可用 | 差：要么 link，要么装 tarball 后无 HMR | 中：仍在同一 Vite 里，但目录归属别扭 | 差：只有产物，无源码 |
| 单二进制内嵌 | 好：一次 `vite build` → `frontendDist` → Tauri 嵌入 | 好 | 好 | 好 |
| 产物体积/tree-shaking | **最好**：单次构建，Rollup 全局 tree-shake，React 单实例 | 好，但预构建包只能保 external，容易带重复 React | 好 | 差：包各自预构建，无法跨包 shake |
| CI 复杂度 | 低：`pnpm install --frozen-lockfile` + 一个 build | 中：需发布/私服/版本提升流程 | 中：Cargo+pnpm 双锁 | 低但脆：清单同步靠脚本 |
| 供应链可控 | 强（lockfile 全覆盖） | 中（依赖外部 registry） | 强 | **弱**（清单可指向任意 URL/产物） |
| 适合谁 | 自家默认包、OEM 需要源码定制 | 已成熟的三方包/闭源 OEM 包 | 强耦合的单体 | 预构建产物分发格式 |

### 1.2 推荐：(a) 为主，(d) 作为"预构建包"输入格式

**目录形态**（`web/` 是唯一 pnpm workspace root）：

```
web/
  pnpm-workspace.yaml
  package.json                    # 只做 workspace 装配 + 脚本
  packages/
    shell-kit/                    # 契约包：Slot 类型、Registry API、Transport 接口、Theme token
    shell/                        # 组装壳：只 import shell-kit + virtual:@zeokeel/registry
    transport-tauri/              # invoke() 实现
    transport-http/               # fetch + WebSocket 实现（serve 模式 / CI）
    transport-mock/               # 无后端 Mock（替代先例里的 window.__TAURI_INTERNALS__ 嗅探）
    ui-chat/                      # 业务包，导出 ./contribute
    ui-composer/
    ui-settings/
    ui-brand-default/
  profiles/
    default.yaml  mock.yaml  oem.yaml
  src/                            # 仅为 vite 的 index.html + main.tsx（引导 3 行）
```

每个包的最小约定（`package.json`）：

```jsonc
{
  "name": "@zeokeel/ui-chat",
  "type": "module",
  "exports": {
    ".":           { "types": "./src/index.ts", "default": "./src/index.ts" },
    "./contribute":{ "types": "./src/contribute.ts", "default": "./src/contribute.ts" }
  },
  "peerDependencies": {
    "react": "^19", "react-dom": "^19", "@zeokeel/shell-kit": "workspace:^"
  },
  "zeokeel": {
    "client": {
      "platform": "web",
      "role": "chat",                                  // 角色由 composer schema 白名单校验
      "baseline": ["react", "react-dom", "@zeokeel/shell-kit"],  // 允许的外部 import（包边界）
      "components": ["core.chat", "core.composer"],    // 本包能提供的 component id
      "inject": ["session", "workspace"]               // 需要宿主提供的上下文服务（编译期检查存在性）
    }
  }
}
```

包边界用**声明 + 检查**而非 bundler 强制（单一构建里没有物理边界）：`baseline` 之外任何 import 由 `scripts/lint-boundaries.mjs`（读 import 图 / rollup metafile）拒绝。闭源/OEM 预构建包（(d) 形态）也遵守同一份 `baseline`，因此可以在不拿到源码的情况下替换 —— 这是"任何包都能换"的关键：**替换的判据是清单 + baseline，不是源码形态**。

### 1.3 "CI 换包"具体怎么换（三层，别只靠 alias）

**第 1 层（主机制）：profile + 生成静态注册表**

`profiles/oem.yaml`：

```yaml
schema: 1
profile: oem
rust:
  plugins: [core, chat, settings]        # Rust 侧聚合 crate 的输入（另一份研究负责细节）
web:
  roles:
    shell:    { package: "@zeokeel/shell" }
    chat:     { package: "@zeokeel/ui-chat" }
    composer: { package: "@zeokeel/ui-composer" }
    brand:    { package: "@acme/zeokeel-brand", version: "1.4.2" }   # ← 唯一改动
  aliases:                                 # 仅当 U 包内部硬 import 了默认实现时用
    "@zeokeel/ui-brand-default": "@acme/zeokeel-brand"
```

```bash
pnpm zeokeel compose --profile oem      # 生成 web/src/generated/*，并 pnpm install --lockfile-only
pnpm zeokeel build   --profile oem      # vite build --mode oem → web/dist/oem
```

composer 做五件事，任何一步失败都退出非零：

1. 解析角色 → `package.json` 的 `zeokeel.client`，校验 `role` 与 profile 一致、`platform: web`；
2. 取 Rust 侧导出的 `ui-contributions.json`（先例已有该产物，见 §2.4），得到 `required: Set<componentId>`；
3. 校验 `required ⊆ ∪providers`、每个 id 恰好一个提供者、`slot` 名在 `shell-kit` 的 `SLOT_DEFS` 里存在；
4. `pnpm install --lockfile-only` 更新锁文件，并把每个角色的 `resolved version + integrity(sha512)` 记录进 `web/src/generated/compose-report.json`；
5. 生成 `registry.gen.ts`（静态 import + 调用 `register()`）、`aliases.gen.ts`、`baseline.gen.ts`。

**第 2 层：pnpm 依赖替换（同 specifier 换实现 / 换到另一个包名）**

```yaml
# web/pnpm-workspace.yaml
packages: ["packages/*"]
catalog:                      # 跨包统一版本，换版本只改一处
  react: "19.3.1"
  typescript: "6.0.3"
```
```jsonc
// profiles/oem/package.json（composer 生成，不手写）
{
  "dependencies": {
    "@zeokeel/ui-brand-default": "npm:@acme/zeokeel-brand@1.4.2"   // pnpm 支持 npm: 别名
  }
}
```

**第 3 层：vite `resolve.alias`（只做兼容垫片）**

```ts
// web/vite.config.ts —— alias 由 composer 生成，禁止手写
import { aliasesFor } from './src/generated/aliases.gen'
export default defineConfig(({ mode }) => ({
  plugins: [react(), zeokeelCompose({ profile: mode })],
  resolve: {
    alias: aliasesFor(mode),                        // 默认 profile 为空对象
    dedupe: ['react', 'react-dom', '@zeokeel/shell-kit'],
  },
  build: {
    outDir: `../dist/${mode}`, manifest: true,
    rollupOptions: { output: { manualChunks: { vendor: ['react', 'react-dom'] } } },
  },
}))
```

> **为什么 alias 不能当主机制**：alias 绕过 pnpm 依赖图与 lockfile —— 产物里会出现"没进锁库的代码"，供应链审计（`compose-report` 的 integrity）失效，且 `tsc` 看到的模块与运行时可能不是同一个。alias 只解决"specifier 名不同"，不提供"这个包被声明过"的证明。

**反模式（明确禁止）**：运行时加载 JS 包（DSH 的 `/plugins` 动态 combo + `window.__DSH_BOOT__` 就是这种，见 §1.4）；`import.meta.glob` 发现模块（构建顺序不确定、无法静态证明包集合）；shell 直接 `import '@zeokeel/ui-chat'`。

### 1.4 DSH 先例：借形状，不借加载方式

- 包清单：`@deepseek-ai/dsh-client-ui-chat/package.json` 有 `"dsh": { "client": { "inject": [11 个包], "platform": "web" } }`，`exports["./client"]` 指向预构建 `lib/client.js`。
- 合成：`@deepseek-ai/dsh-client-modules` 的 Node 半 **在宿主启动时** 扫描 Loader 条目、合成 boot graph、把 `WebBootEntry` 注入 `window.__DSH_BOOT__`，并以 `/plugins/??...&rev=` combo 路由按需投递 bundle；HMR 换 revision。这是**运行时组装**，与"CI 阶段换实现"直接冲突，**不能照抄**。
- 但它的**数据形状**值得抄：`dsh.client` 清单、`inject`（依赖声明）、`platform`、以及 `PLATFORM_MODULES` 共享模块基线（所有动态 bundle 的 external 只能解析到这张冻结表 —— 等价于我们的 `baseline`）。
- 前端 dist 本身就是独立包：`@deepseek-ai/dsh-web-frontend` 只 `exports: { "./dist/*": "./dist/*" }`，由 `dsh web` 托管。对应我们的 `web/src/generated` + `frontendDist`。
- 载体双形态：`@deepseek-ai/dsh-host-webserver`（路由/索引注入/静态 fallback/compression）+ `@deepseek-ai/dsh-host-frontend-static`（`distIndex`、token→cookie 鉴权、`<base href="./">`、目录穿越 403）证明"UI = 静态产物 + 一个很小的 HTTP 载体"是成熟形态；Tauri 的 `fetchBundle()` 式 shell-owned carrier 在 DSH 里也有对应物。→ 支撑 §4 的"同一 dist 双载体"。

---

## 2. 贡献/槽位模型

### 2.1 分工：Rust 声明"贡献"，TS 包声明"提供"

| 侧 | 声明什么 | 为什么 |
|---|---|---|
| Rust 插件（cordis plugin） | `UiContribution { slot, component, order, params }` + `UiKind`（Root/Page/Settings/Command/ToolRenderer…） | 插件启停是内核语义：禁用插件 → 贡献消失，无需重建前端（先例 `scripts/plugin-matrix.sh` 已按此断言） |
| TS 包 | `components: string[]`（id → 组件在本包内实现）+ `./contribute` 入口 | 换 UI 包只影响"谁提供 id"，不影响"哪里要组件" |
| profile | 角色 → 包 | shell 不认识包名，只认识角色与 id |

这条分工让三件事同时成立：**运行时启停**（Rust 贡献按启用集合过滤）、**构建期可替换**（提供者可换）、**shell 零业务 import**。

### 2.2 shell 如何做到不 import 业务组件

```tsx
// web/packages/shell/src/App.tsx
import { SlotOutlet, useContributions } from '@zeokeel/shell-kit'
export function App() {
  const root = useContributions().single('app.root')     // 来自 Rust 的贡献描述符
  if (!root) return <BootDiagnostic reason="missing-root" />
  return <SlotOutlet contribution={root} />              // 内部查注册表，不 import 任何业务包
}
```

- 唯一动态入口是 `virtual:@zeokeel/registry`（Vite 虚拟模块，内容来自 composer）。`shell-kit` 提供 `register/get/ids`，类似先例 `client/web/src/ui/registry/api.ts` 的 `Map<string, ComponentType>`。
- 未注册 id **不是崩溃**：渲染可见诊断卡片 + 记入 boot audit（先例 `UnknownComponent.tsx` / `audit.ts` 的做法；空窗无法区分"加载中"和"缺贡献"）。
- 强制：`eslint no-restricted-imports`（shell 目录禁 `@zeokeel/ui-*`、`@acme/*`），外加 `scripts/lint-boundaries.mjs` 在 CI 里检查 rollup metafile 中 shell chunk 的 import 边。

### 2.3 构建期生成注册表：实现选择

| 方式 | 结论 |
|---|---|
| Vite 插件发**虚拟模块** `virtual:@zeokeel/registry` | **推荐**：每次构建必刷新，无漂移；`tsc --noEmit` 用一份 `declare module` 声明（导出 `ShellModule[]`）即可通过 |
| 生成并提交 `registry.gen.ts` + `--check` | 先例做法（`scripts/gen-plugin-graph.py --check`），适合需要 diff 审计的场合；对 TS 侧我们只在 `--emit` 审计模式下写文件 |
| `import.meta.glob` | 禁用（发现顺序/包集合不可静态证明） |

生成物示例（虚拟模块的等价产物）：

```ts
// 概念输出；不落盘，除 --emit
import { register } from '@zeokeel/shell-kit'
import * as chat from '@zeokeel/ui-chat/contribute'
import * as composer from '@zeokeel/ui-composer/contribute'

const mods = [chat, composer]                 // 顺序 = profile 角色顺序（确定性）
for (const m of mods) register(m)             // register 必须幂等（HMR 重复调用）
export const provided = new Set(mods.flatMap(m => m.meta.components))
```

确定性要求与先例一致：顺序按 `(profile 角色顺序, 包名)` 排序，**不依赖文件系统枚举**（先例 `gen-plugin-graph.py` 用 `(group, crate)` 排序并显式 `PIN_FIRST` 固定语义顺序，同样的坑）。

### 2.4 契约产物：复用先例的三份 JSON

先例 `scripts/export-contracts.sh` 已从源码抽取并提交：
`shared/contracts/ipc-commands.json`（所有 `#[tauri::command]`）、`shared/contracts/ui-contributions.json`（`UiKind`/字段/所有 `component:` 字面量，含 `pub const` 常量解引用）、`shared/contracts/ui-slots.json`（从 `SLOT_DEFS` 解析 slot 名与 kind）。新项目把这三份升级为 **composer 的输入**（并在 CI `--check` 防漂移），即"Rust 是贡献的唯一真源，前端生成物不得手改"。

槽位（slot）本身的形态建议保持先例三种：`single` / `list`（有序，带 `order` + tie-break）/ `keyed`（同 key 后注册覆盖，用于主题/品牌这类"替换语义"）。

---

## 3. Rust ↔ TS 类型同步

### 3.1 三类需要同步的面（不要用一套方案硬套）

| 面 | 例子 | 同步方向 | 推荐 |
|---|---|---|---|
| A. 命令/请求-响应 DTO | `ipc-commands.json` 里的每个 command 参数与返回值 | Rust→TS | ts-rs |
| B. 服务端事件/流 | 会话流、工具调用增量、审批请求；形态是 `{ type, payload }` 判别联合 | Rust→TS | ts-rs（internally tagged enum 支持好） |
| C. 工具参数 Schema | rig `Tool::definition` 的 `parameters: serde_json::Value` | Rust→JSON Schema（模型侧本来就要） | schemars；TS 类型可选由 `json-schema-to-typescript` 派生 |
| D. 极小的稳定信封 | `{ ok: true } | { ok: false, error }` | 手写 | 手写（生成收益低于噪音） |

### 3.2 决策矩阵

| 方案 | 单一真源 | 枚举/泛型保真 | 生成触发 | CI 防漂移 | 运行时校验 | 额外依赖 | 结论 |
|---|---|---|---|---|---|---|---|
| **ts-rs 12.0.x** | Rust | **好**（serde tag/content/rename、Option、HashMap、泛型、递归需 `#[ts(recursive)]`） | `cargo test export_bindings`（推荐）或 `TS::export_to_string()` | `git diff --exit-code` | 无（只出类型） | 仅 proc-macro | **主方案** |
| typeshare 1.0.x | Rust | 中（支持 tagged enum、部分泛型；多语言目标但 TS 是其一） | 外部 CLI `typeshare` | 同上 | 无 | 额外二进制 | 不推荐（多语言目标对我们是负担） |
| schemars → JSON Schema → quicktype / json-schema-to-typescript | Rust | 好（schema 是 serde 语义） | 两步管道 | 两步都可能静默降级 | **有**（JSON Schema/zod） | 3 个工具 | 只用于 C 面 |
| 手写 | 无 | — | 人 | 靠 review | 无 | 无 | 只用于 D 面 |

**推荐：ts-rs 为主 + schemars 用于工具参数**。理由：
1. ts-rs 直接吃 serde 属性，生成的是**我们实际序列化后的形状**，枚举判别联合正是事件流需要的；
2. 唯一低成本的漂移闸门是 `cargo test` + `git diff --exit-code`（与先例 `--check` 文化一致）；
3. 工具参数本来就必须是 JSON Schema（喂给模型），schemars 是顺带产出，不引入额外真源。

**落地片段**

```rust
// crates/zeokeel-contract/src/lib.rs
use ts_rs::TS;
#[derive(serde::Serialize, serde::Deserialize, TS)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
#[ts(export, export_to = "../../web/packages/shell-kit/src/generated/")]
pub enum HostEvent { TurnStarted { .. }, Delta { .. }, ApprovalRequested { .. } }
```

```bash
# 生成 + 漂移校验（CI 与本地同一命令）
TS_RS_EXPORT_DIR=web/packages/shell-kit/src/generated \
  cargo test -p zeokeel-contract --features ts-export export_bindings
git diff --exit-code -- web/packages/shell-kit/src/generated   # 生成物未提交/漂移则失败
npx tsc -p web/packages/shell-kit --noEmit                      # 生成物必须被 TS 编译通过
```

成本注意：ts-rs 的 `export` 依赖测试；若无条件 `#[ts(export)]`，所有 crate 的 `cargo test` 都会写文件（干扰并行）。**做法**：用一个 feature `ts-export` 包住，默认关闭；只有 `zeokeel-contract` 打开。

**运行时校验要不要？** 前端对**不可信**输入（OEM 预构建包、远程 serve 模式的浏览器）需要校验，但不必给所有 DTO 上 zod：只在 transport 边界（B 面事件）用 `json-schema-to-zod` 从同一 schema 生成一个 envelope 校验器，其余靠 TS。

---

## 4. 桌面壳选型

### 4.1 决策矩阵

| 维度 | Tauri 2.12.x | wry 0.57 + tao（自建） | 内嵌 HTTP + 系统浏览器 | egui / slint |
|---|---|---|---|---|
| 单二进制 | **是**（`frontendDist` 编译期嵌入，`tauri build --no-bundle` 出单文件） | 是 | 否（二进制 + 用户浏览器） | 是 |
| 跨平台 webview 一致性 | 中：WKWebView / WebView2 / WebKitGTK 三引擎 | 中：同三引擎 | **差**：再加 Chrome/Firefox/Safari，版本不可控 | **好**：自绘，一致 |
| 前端包化可行性 | **高**：dist 是纯静态资源，composer 与壳无关 | 高 | **最高**：纯静态服务器，无 IPC 耦合 | 无（放弃 npm/React 生态） |
| 远程访问 | 中：需另起本地 server + 鉴权（可做，见 §4.2） | 中 | **原生** | 差 |
| 体积（量级估算，release+strip） | 二进制 3–8 MB；Linux AppImage 因带 webkit 依赖 70–120 MB | 二进制更小，运行时依赖同上 | 20–40 MB（axum+tower+rustls） | 5–20 MB |
| 签名/公证 | **现成**：macOS notarytool + Windows Authenticode + updater minisign 配置化 | 全自建 | 基本不需要 | 全自建 |
| CI 无头测试 | 中：`tauri-driver` **仅 Windows/Linux**；macOS 官方建议 CrabNebula 付费跨平台 fork（见 [WebDriver 文档](https://v2.tauri.app/develop/tests/webdriver/)） | 差：无官方 driver | **最好**：Playwright 直连 localhost / curl | 差：截图对比 |
| 权限/能力模型 | **强**：capabilities + 命令白名单 + CSP 注入 | 无（自己写） | 自建（token/cookie/CSRF） | 无 |
| 生态与人力 | **最好**（2.12.5，2026-10-21；插件/打包/更新齐全） | 中（自己维护 IPC/更新/打包） | 好（axum/axum-extra） | 中 |
| 与"组件是包"的契合 | 高 | 高 | 高 | **不契合** |

### 4.2 推荐：Tauri 2 主壳 + 同一 dist 的 `zeokeel serve` 副载体

理由（按权重）：

1. **单二进制与分发**是硬要求 → 只有 Tauri / wry / egui·slint 满足；egui·slint 与"前端组件是 npm 包"不可兼容，出局。
2. **wry 自建** = 重写 Tauri 已经做对的那些事（IPC、能力白名单、CSP 注入、打包、签名、更新），没有收益；只有在"不要 Tauri 的 Rust 依赖树"时才有意义，而我们不介意。
3. **纯浏览器方案**丢掉单二进制与窗口/托盘等桌面能力，但它的两个优点（远程访问、CI 极便宜）值得保留 → 所以做成**第二个载体**，而不是另一个产品。
4. DSH 的 `host-webserver` + `host-frontend-static` 已经证明"静态 dist + 极小 HTTP 载体 + token→cookie 鉴权"可产品化；我们把同一份 dist 同时喂给 Tauri 的 `frontendDist` 和 `serve` 载体。

**双载体的实现约束**（关键，别做成两套前端）：

```rust
// crates/zeokeel-app/src/main.rs
#[cfg(feature = "desktop")] fn main() { tauri::Builder::default()/* … */ .run(..) }  // dist 由 frontendDist 嵌入
#[cfg(feature = "serve")]   fn main() { zeokeel_serve::run(include_dir!("$ZEOKEEL_DIST")) }
// include_dir! 支持 $VAR 环境变量插值；若想用 build.rs 产物路径，改用 rust-embed。
// 两个 feature 互斥：同一份 web/dist/<profile>，两种进二进制的方式，不会重复打包。
```

- 前端只有一个 transport 抽象：

```ts
// web/packages/shell-kit/src/transport.ts
export interface Transport {
  invoke<Req, Res>(cmd: string, req: Req): Promise<Res>
  subscribe(handler: (e: HostEvent) => void): () => void
}
```
  - `transport-tauri`：`@tauri-apps/api` 的 `invoke` + `listen`；
  - `transport-http`：`fetch('/rpc/<cmd>')` + `WebSocket('/events')`（serve 模式）；
  - `transport-mock`：profile=`mock` 时注入（**替代**先例里 `window.__TAURI_INTERNALS__` 缺失就回退 mock 的隐式嗅探 —— 先例 `client/playwright.config.ts` 注释确认这曾是 Playwright 能跑的原因，我们要把它显式化为 profile）。
- 选择在构建期由 profile 决定（`RUNTIME_TRANSPORT` 常量注入），不在运行时嗅探 —— 保持"构建期决定"的一致原则。

**serve 模式的安全底线**（因为一旦开就是 RCE 面）：
- 默认只绑 `127.0.0.1`，端口 0（随机）并把 URL 打到 stderr；
- 首次访问要求一次性 token（`?token=` → 303 Set-Cookie），之后 cookie 鉴权（先例 `dsh-host-frontend-static` 的 `ctx.connection.authorizeIndex` 就是这个模型）；
- 索引响应用 `renderIndex` 注入 `<base href="./">`，静态资源公开、目录穿越 403；
- `--listen 0.0.0.0` 必须显式开启且强制 TLS（selfsigned 或用户证书）+ 来源检查。

**Tauri 配置片段**（CSP 与 capabilities 是这个方案最容易踩的地方）：

```jsonc
// crates/zeokeel-app/tauri.conf.json
{
  "build": { "frontendDist": "../../../web/dist/default", "devUrl": "http://127.0.0.1:1420" },
  "app": {
    "security": {
      // 构建期 Tauri 会为本地脚本追加 hash、为样式/外部脚本追加 nonce
      "csp": {
        "default-src": "'self'",
        "connect-src": "ipc: http://ipc.localhost",
        "img-src": "'self' data: blob:",
        "style-src": "'self' 'unsafe-inline'"
      },
      "devCsp": null,
      "capabilities": ["main"]
    }
  }
}
```

> 若改用 `serve` 模式让 webview 加载 `http://127.0.0.1:<port>`（换取"Tauri 与浏览器同一路径"），必须在 capability 里配 `remote.urls` 才允许 IPC（Tauri v2 schema 的 `Capability.remote` 字段），否则 `invoke` 静默失败。**建议默认不要这么做**：桌面模式走内嵌 `tauri://`，只让 serve 模式跑 HTTP。

Tauri 官方明确"不创建 localhost HTTP 服务器来服务 webview"（[tauri README](https://github.com/tauri-apps/tauri)），所以两载体是并列的，不是"Tauri 内部再套服务器"。

---

## 5. CI 落地草案

### 5.1 本地/CI 同一套命令

```bash
# web/
pnpm install --frozen-lockfile
pnpm zeokeel compose --profile "$PROFILE" --emit    # 生成 registry/alias/report；--emit 走可审计模式
pnpm typecheck                                      # tsc -b（含 shell-kit 生成类型）
pnpm build --profile "$PROFILE"                     # vite build --mode "$PROFILE"
node scripts/verify-dist.mjs --profile "$PROFILE"   # 产物校验，见 5.3
pnpm vitest run                                     # 组件/契约单测（jsdom）
pnpm exec playwright test --project="$PROFILE"       # 冒烟（transport-mock 或 serve 载体）
```

`pnpm zeokeel` 指 `web/tools/compose-cli`（`bin` 暴露 `zeokeel`）；profile 也可用 `ZEOKEEL_PROFILE`。

### 5.2 Playwright：按 profile 分项目，跑构建产物

```ts
// web/playwright.config.ts
const PROFILES = ['default', 'mock', 'oem'] as const
export default defineConfig({
  testDir: './tests/e2e',
  workers: 1, retries: 0, forbidOnly: !!process.env.CI,
  webServer: {
    command: 'node tools/preview.mjs',       // 静态服务 web/dist/<profile>，随机端口
    url: `http://127.0.0.1:${process.env.PW_PORT ?? 4173}`,
    reuseExistingServer: !process.env.CI,
  },
  projects: [
    ...PROFILES.map((p) => ({
      name: p,
      use: { ...devices['Desktop Chrome'], baseURL: urlFor(p) },
      metadata: { profile: p },
    })),
    // 近似 macOS 的引擎覆盖：Linux 上用 WebKit 跑 default 冒烟
    { name: 'webkit-default', use: { ...devices['Desktop Safari'], baseURL: urlFor('default') } },
  ],
})
```

要点：
- **不依赖 Tauri 二进制**跑 E2E（macOS 上官方 driver 收费/不可用），把"壳"只在 `e2e-tauri` 这个可选 job 里测；
- `mock` profile 的测试不得依赖网络/模型（先例原则：无 provider、无真实后端）；
- 每个 profile 至少一条"启动 → 根槽位渲染 → 关键交互 → 无 console error"冒烟；
- 失败留 trace/screenshot 到 CI artifact（先例 `.review/playwright/`）。

### 5.3 产物校验（`verify-dist.mjs` 必做项）

| 检查 | 失败含义 |
|---|---|
| `index.html` 不含 `http(s)://` 外链资源 | 离线/内嵌会白屏 |
| `manifest.json` 里每个 chunk 的 sha256 写入 report | 审计/可复现 |
| React 在 metafile 只出现一次（`node_modules/react/` 计数=1） | 双实例会导致 hooks 崩溃 |
| shell chunk 的 import 边不含 `@zeokeel/ui-*` / `@acme/*` | 包边界破了 |
| `required ⊆ provided`（来自 compose-report） | 换包漏了 id → 运行时诊断卡片 |
| 预构建包的 import ⊆ `baseline` | 闭源包夹带、锁文件外代码 |
| 体积预算（default：JS gzip ≤ 2.5 MB，可调） | 依赖树膨胀 |
| `default` 产物中不含 OEM 私有标识 / `oem` 产物中含 | 证明"换包"真的换了 |
| CSP 头（serve 模式）与 `tauri.conf.json` 同源生成 | 两份策略漂移 |

### 5.4 GitHub Actions 骨架

```yaml
jobs:
  web:
    strategy:
      fail-fast: false
      matrix: { profile: [default, mock, oem] }
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
      - uses: actions/setup-node@v4
        with: { node-version: 24, cache: pnpm, cache-dependency-path: web/pnpm-lock.yaml }
      - run: pnpm install --frozen-lockfile
      - run: pnpm zeokeel compose --profile ${{ matrix.profile }} --check   # 防生成物漂移
      - run: pnpm typecheck
      - run: pnpm build --profile ${{ matrix.profile }}
      - run: node scripts/verify-dist.mjs --profile ${{ matrix.profile }}
      - run: pnpm vitest run
      - run: pnpm exec playwright test --project=${{ matrix.profile }}
      - uses: actions/upload-artifact@v4
        with: { name: web-${{ matrix.profile }}, path: web/dist/${{ matrix.profile }} }
      - uses: actions/upload-artifact@v4
        with: { name: compose-report, path: web/src/generated/compose-report.json }
  contract:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
      - run: cargo test -p zeokeel-contract --features ts-export export_bindings
      - run: git diff --exit-code -- web/packages/shell-kit/src/generated shared/contracts
  desktop-smoke:            # 可选，慢，仅 Linux
    needs: [web]
    runs-on: ubuntu-24.04
    steps:
      - run: sudo apt-get install -y libwebkit2gtk-4.1-dev xvfb
      - run: cargo tauri build --no-bundle --features desktop
      - run: xvfb-run -a ./target/release/zeokeel --version   # 单二进制冒烟即可
```

> 取舍说明：`tauri-driver` 的 WebDriver 冒烟只在 Linux/Windows 值得做（macOS 要付费 fork），且一个"能启动 + 能渲染根槽位"的 Xvfb 冒烟已经覆盖了 90% 的壳级事故；真正的交互回归交给 HTTP 载体上的 Playwright。

---

## 6. 风险清单

### 6.1 CSP
- Tauri 构建期自动给本地脚本加 hash、给样式加 nonce；`devCsp` 未设时沿用 `csp`（[CSP 文档](https://v2.tauri.app/security/csp/)）。→ 任何 `<script>` 内联（例如主题首帧）都必须被 hash 覆盖；`style-src 'unsafe-inline'` 是现实妥协（Tailwind/内联样式）。
- `connect-src` 必须含 `ipc: http://ipc.localhost`，否则 `invoke` 失败（Windows 用 `http://ipc.localhost`，macOS/Linux 用 `ipc:`）。
- serve 模式没有 Tauri 的自动注入 → **CSP 必须由我们的 HTTP 层输出，且与 `tauri.conf.json` 从同一份源生成**，否则两载体安全等级不一致。
- 预构建 OEM 包若内联 `<style>`/`<script>`，构建期 hash 会变 → 计入 compose-report，否则运行时被 CSP 拦。

### 6.2 webview 跨平台差异（最大风险）
- 三引擎三版本：WKWebView 跟 OS、WebView2 自动更新、**WebKitGTK 跟发行版**（Ubuntu 22.04 vs 24.04 的 4.1 版本差可以到两年）。
- 具体高危特性：`color-mix()`、`:has()`、`@container`、`popover`、`<dialog>::backdrop`、View Transitions、`showSaveFilePicker`（WKWebView/WebKitGTK 缺失）。→ 规则：新 CSS 特性必须 feature-detect（`@supports`），选文件一律走 Tauri dialog 插件而不是 Web API。
- 兜底：Playwright 增加 `webkit` project；`browserslist` 目标钉 `safari 16 / chrome 110 / firefox 110`；CSS 用 `lightningcss` 或 tailwind 的兼容层编译。
- 不一致的隐性来源还有**字体/度量**：截图回归（先例有 `docs/screenshots/`）用固定 viewport + 字体 fallback，别拿像素差当硬门（软门 + 人工）。

### 6.3 npm 供应链与 lockfile
- `pnpm-lock.yaml` 必须提交 + CI `--frozen-lockfile`；`compose-report.json` 记录每个角色的 `integrity`，发布 job 断言 = 构建 job。
- 供应链加固：`minimumReleaseAge`（先例 `pnpm-workspace.yaml` 已经用了 `minimumReleaseAgeExclude`，说明该策略在实践中被采用）+ `onlyBuiltDependencies` 白名单（禁任意 postinstall）+ `osv-scanner` 每次 release 跑一次（不阻塞日常 PR）。
- **禁止**在 `default` profile 里出现 `npm:` 别名或 tarball/URL 依赖（那是 OEM 的豁免区，且要写进 compose-report 的 `exemptions`）。
- 闭源预构建包是供应链最弱环：它绕过源码审查，只有 `baseline` + integrity 两道闸 → OEM 场景应要求提供 SBOM（`pnpm licenses`/cyclonedx）。

### 6.4 离线构建
- 需要：`pnpm fetch` 预热 store + `pnpm install --offline`；`pnpm-lock.yaml` 提交；Playwright 浏览器预装（`PLAYWRIGHT_BROWSERS_PATH` 指向缓存）；Cargo `vendor` 或内网 registry。
- 风险点：`pnpm` 的 store 与 workspace 硬链接在 CI 缓存恢复后要 `pnpm install --frozen-lockfile --offline` 验证；`minimumReleaseAge` 与离线镜像里"已发布但未同步"的版本会冲突 → 离线镜像保留版本清单。
- Tauri 侧：`cargo tauri build` 会尝试下载 NSIS/WiX/AppImage 工具链 → 必须预置（`TAURI_BUNDLER_*` 缓存）或 `--no-bundle` + 平台 CI。

### 6.5 HMR 与包边界
- 源码包跨包 HMR 正常（Vite 直接读 source）；**换 profile 时必须整页重载**：watch `profiles/*.yaml` 与生成的 `registry.gen.ts`，变化时 `server.restart()`（否则注册表是旧的，旧包组件还挂着）。
- `register()` 必须**幂等 + 无顶层副作用**：HMR 会重复执行；`register` 语义"后注册覆盖"（先例 `api.ts` 注释即此约定），但同一 id 两个包注册要在 CI 报错而不是运行时悄悄覆盖。
- React Fast Refresh 要求单实例：`peerDependencies: react ^19` + `resolve.dedupe` + metafile 断言（§5.3）。
- 虚拟模块与 codegen 的坑：`tsc` 看不到虚拟模块的真实内容 → 类型契约必须全在 `shell-kit`（`SlotDefs`、`UiModule`、`ComponentProps`），生成的只是"绑定表"；不要在虚拟模块里发明类型。
- CSS/主题边界：token 只能由 theme/brand 包提供；业务包禁止 import tailwind preflight（否则换 brand 包会重复重置样式）。i18n 命名空间 = 包名，避免 key 冲突。

### 6.6 其它值得预埋的
- **贡献描述符版本**：Rust `UiKind` 增枚举值 = 前端 break。给 `ui-contributions.json` 加 `schema_version`（先例已有 `schema_version: 1`），前端对未知 kind 渲染"不支持"卡片而不是崩。
- **slot 名演进**：`SLOT_DEFS` 是唯一真源，先例 `export-contracts.sh` 已经把它导出成 `ui-slots.json` —— 让 Rust 侧在编译期/测试期校验声明的 slot 存在，别等到运行时诊断卡片。
- **双载体分歧**：`serve` 模式与桌面模式的行为差异要用同一份 Playwright 用例覆盖（同一 dist、两个 transport），否则会退化成两个产品。

---

## 附录 A：与 DSH / ZeoKeel-Next 的取舍一览

| DSH 做法 | 是否采用 | 改成什么 |
|---|---|---|
| `dsh.client` 清单（platform/inject/external） | ✅ 采用 | 叫 `zeokeel.client`，字段加 `role`/`components` |
| 运行时 boot graph + `/plugins` combo 投递 | ❌ 不采用 | 构建期 composer 生成静态 registry |
| `PLATFORM_MODULES` 冻结共享基线 | ✅ 采用 | 叫 `baseline`，由 lint + metafile 校验 |
| HMR 按 revision 换 bundle | ❌ 不采用 | dev 用 Vite HMR；profile 变化整页重载 |
| `host-webserver` + `host-frontend-static` | ✅ 采用 | `zeokeel serve` 载体（axum），token→cookie |
| `dist` 本身是包（`dsh-web-frontend`） | ✅ 采用 | `web/dist/<profile>` 作为构建产物边界 |
| ZeoKeel-Next `UiContribution`(Rust) + TS registry | ✅ 采用 | 加上"提供者包"清单 + profile 角色 |
| `modules.generated.ts` + `--check` | ✅ 部分采用 | TS 侧走虚拟模块；Rust 侧生成物仍提交 + `--check` |
| `export-contracts.sh` 三份 JSON | ✅ 采用并升级 | 变成 composer 输入 |
| `window.__TAURI_INTERNALS__` 嗅探回退 mock | ❌ 不采用 | 显式 `mock` profile + `transport-mock` |

## 附录 B：待验证项（本轮未实测，建议原型期 30 分钟内打掉）

1. Vite 8 / 虚拟模块 + `tsc -b` 的组合在老版本 TS 上是否需要对 `declare module` 做额外配置（本机先例用 TS 6.0.3，未见问题）。
2. Tauri 2.12 的 `tauri build --no-bundle` 在 Linux 无头环境下是否需要 `libwebkit2gtk-4.1-dev`（本文按官方 prerequisites 假设需要）。
3. `pnpm catalog` + `--frozen-lockfile` 在"只改 profile 不改版本"时是否仍会因 lockfile 的 `specifiers` 变化而失败（若有，compose 里加 `--lockfile-only` 再 `--frozen-lockfile` 的两段式）。
4. Playwright `webkit` project 在 Ubuntu 上对 WebKitGTK 的近似度（只是近似，不能替代真机）。
