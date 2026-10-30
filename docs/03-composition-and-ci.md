# 03 · 组合与 CI 替换机制（核心）

> 这份文件回答唯一一个硬问题：**Rust 静态链接、内核明确不做动态加载的前提下，"一切皆包" 和
> "所有包都能在 CI 阶段被替换" 怎么成立？**
> 答案：profile 描述组合 → `zk` 解析并生成粘合代码 → CI 对每个 profile 各构建一次并跑契约/换包矩阵。
> 证据：`experiments/composition-poc/` 已实测三种实现换同一角色，含 **workspace 外**实现（形状等同外部包；真正仓外 git 源见 §6.5）。

## 1. 三种可选路线，以及为什么选构建期组合

| 路线 | 机制 | 换包是否需要重建 | 单二进制 | 与 cordis 冲突 | 结论 |
| --- | --- | --- | --- | --- | --- |
| A. 构建期组合 | profile + 代码生成 + 原生静态链接 | **是** | ✅ | 无 | **采用** |
| B. Cargo features | `impl-fs-sandbox` 之类 feature 开关 | 是 | ✅ | 无 | 否，见 §2 |
| C. 动态库 | `dylib` / `abi_stable` / 插件 `.so` | 否 | ❌（要带若干 .so） | 与"无 ABI、无 unsafe、静态链接"直接冲突 | 否 |
| D. WASM 运行时 | 包编译成 wasm，host 用 wasmtime 加载 | 否 | ✅（宿主持 wasm） | 内核说隔离属于 embedder，可行但要自建 ABI + 沙箱 + 权限 | 留作 M5 扩展点（仅叶子能力） |

选 A 的核心理由：它把"可替换"变成**编译期可证明的属性**，而不是运行时约定。具体到三条收益：

1. 换实现后若契约不匹配，`cargo build` 直接失败（POC 实测），不会等到用户点开某个面板才 500。
2. 单二进制、无 `.so` 分发、无 ABI 版本矩阵，符合 cordis 内核设计前提。
3. 组合矩阵可以被 CI 穷举——这就是"实现类包都能在 CI 阶段替换"的可执行定义。

代价（必须写进产品预期）：换实现要重新构建；不能像 DSH 那样改 YAML 重启即生效。
这条代价是 ADR-001，后续任何"运行时热插拔"需求都要先推翻它。

## 2. 为什么不用 Cargo features 做替换

feature 看起来最省事，但在"一切皆包 + 可替换"下会塌：

| 问题 | 说明 |
| --- | --- |
| 表达能力不足 | feature 只能表达"开/关某个编译分支"，表达不了"角色 `tool.fs` 由哪个包提供"、"基数 singleton/multi"、"UI facet 一起换"。 |
| feature 统一（unification） | 同一个 crate 被两个依赖方用不同 feature 集合引入时，Cargo 取并集。profile A 想关掉的能力会被 profile B 的依赖悄悄打开，**组合不可预测**。 |
| 组合爆炸 | N 个可替换角色 × 每个 2 个实现 = 2^N 个 feature 组合，无法穷举，也无法审计。 |
| 换包不是加 feature | 第三方包不可能预先在你的 crate 里埋 feature。 |
| 前端无对应物 | 前端没有 feature，得再造一套机制，违背"统一一套组合语义"。 |

feature 仍有一个合法用途：**单个包内部**的可选依赖裁剪（如 `plugin-mcp` 只在启用时编译 rmcp），
这属于包实现细节，不属于组合层。

> 实现细节以 [`24-composition-and-runtime-spec.md`](24-composition-and-runtime-spec.md) 为准。

## 3. 组合输入：edition × profile × channel 叠层

`editions/*.toml` 定义包集合（含 `exclude`），`profiles/*.toml` 只做环境覆盖，`channel` 定品牌/默认值；业务源码里不出现包名。

v3 的组合输入是**三层叠加**，不是单一文件：

```text
edition（企业版/个人版：包含哪些包、产物形态、许可；enterprise 继承 personal + exclude）
  └─ profile（环境：default/mock/minimal/ci-matrix；覆盖角色/设置）
       └─ channel（official / beta / oem-<customer>：品牌/文案/默认值/服务器地址；可带额外包）
```

```toml
# editions/personal.toml —— 基线（客户端全部本地包）
[edition]
name    = "personal"
targets = ["desktop"]

# editions/enterprise.toml —— 个人版 + 企业管控包
[edition]
name         = "enterprise"
base_edition = "personal"                    # 继承个人版包集合
targets      = ["server", "desktop", "admin"] # 追加服务端/管理台
exclude      = ["zeokeel/client-provider-settings", "zeokeel/client-update-github", "zeokeel/client-onboarding-personal"]  # 排除个人版专属包

[packages]
# 追加企业管控包（示例）：登录、网关、商店客户端、服务端域
"auth.client"           = "zeokeel/client-auth"
"llm.provider.default"  = "zeokeel/llm-gateway"            # 覆盖个人版本地 provider
"client.update"         = "zeokeel/client-update-server"   # 覆盖 GitHub Releases
"srv.auth.source"       = "zeokeel/srv-auth-local"
"srv.wallet"            = "zeokeel/srv-wallet"
"srv.gateway"           = "zeokeel/srv-gateway"
"srv.skills"            = "zeokeel/srv-skills"

# 约束：企业版不得复制客户端基线；必须能表示为 (personal − exclude) + 追加/覆盖。
# CI 用 `zk plan --edition enterprise` 断言：
#   ① 除被覆盖角色外，个人版包集合 ⊆ 企业版闭包；
#   ② 企业版闭包不包含任何 personal-only 包（provider 设置 UI、GitHub 更新器等）。

# profiles/default.toml
[profile]
name  = "default"
base  = "base"

[packages]
# 角色 = 包 id（角色映射唯一表；channel 追加/覆盖仍走同一套校验）。singleton 角色恰好一项；multi 角色写数组。
"platform.shell"       = "zeokeel/platform-tauri"
"platform.http"        = "zeokeel/host-webserver"      # 指定了才有 serve 载体
"ui.shell"             = "zeokeel/ui-shell"
"ui.transport"         = "zeokeel/transport-tauri"
"agent.loop"           = "zeokeel/agent-loop"
"store.session"        = "zeokeel/store-session-sqlite"
"tool.fs"              = "zeokeel/tool-fs"
"tool.shell"           = "zeokeel/tool-shell"
"tool.web"             = "zeokeel/tool-web"

# multi 角色用数组；顺序即渲染/激活顺序
"ui.slot.settings.section" = [
  "zeokeel/ui-settings-general",
  "zeokeel/ui-settings-models",
]

[settings]
# 与包无关的产品级设置；包内设置由包的 [config].schema 约束
theme = "auto"
```

规则：

1. **`edition`/`profile` 的 `[packages]` 是唯一角色映射表**：角色→包 id，singleton 一项、multi 数组；同一 singleton 出现两次 = 错误。
   UI 角色、平台角色、transport 角色都在同一张表里（facet 原子替换，见 ADR-011）。
   `channel` 只能注入品牌/默认值，或按 deep-OEM 声明**追加/覆盖**包；channel 的包仍走同一角色/契约校验，不得绕过。
2. **profile 里不写版本、不写路径**：版本/源/哈希一律由 `zk.lock` 解析（§5）。
3. **没有 `[overrides]` 之类的第二组合通道**：换包就是改 `[packages]`；CI 的换包矩阵也是生成一份
   完整 profile（或同一 profile 的临时副本），不引入旁路语义。
4. `base` 继承只允许**单链**（`default → base`），不做多继承。
5. profile 不允许写"启用/禁用某段业务代码"——那说明该能力还没包化。

典型 profile 集合：

| profile | 用途 | 差异 |
| --- | --- | --- |
| `base` | 公共底座 | 契约、runtime、host 能力 |
| `default` | 生产桌面 | 真实 provider、真实存储、真实 fs |
| `mock` | e2e / 离线 | 确定性 LLM、内存存储、假 fs、假时钟 |
| `minimal` | 最小可启动 | 只有 chat + fs，用于证明"裁剪不崩" |
| `hardened` | 安全模式 | `tool-fs-sandboxed`、`tool-shell-restricted`、审批强制 |
| `ci-matrix` | CI 矩阵 | 各 edition × channel × 关键变体组合 |
| `oem-<vendor>` | 白标 | 换 brand/theme/default provider（channel 层） |
| `nightly` | 实验 | 包含 `stability = "experimental"` 的包 |

## 4. `zk` CLI：解析器 + 生成器 + 校验器

一个 workspace crate（`tools/zk`），也发布成二进制供第三方使用。命令面：

| 命令 | 作用 | CI 用途 |
| --- | --- | --- |
| `zk validate` | 清单/契约/角色/基数/依赖图校验；契约 crate 去重 | PR 必跑 |
| `zk compose --edition E --profile X --channel C` | 生成所有粘合产物（§5） | 构建前置 |
| `zk compose ... --check` | 不写文件，断言生成物无漂移 | PR 必跑（防手改生成物） |
| `zk verify-lock` | 断言 CI 内重解析结果与 `zk.lock` 一致 | 变体构建 |
| `zk db lock` | 重新计算迁移内容哈希（只允许新增） | 迁移变更 |
| `zk api` | 生成 api-catalog/openapi/权限矩阵 | 契约变更 |
| `zk oem add <customer>` | 在索引仓登记客户并挂 submodule（需私有仓权限） | OEM 接入 |
| `zk plan --edition E --profile X --channel C` | 输出将编译的包、角色、预计产物 | 评审/诊断 |
| `zk explain <role>` | 该角色所有候选实现、契约、被谁 require | 替换时查 |
| `zk diff --edition A --profile X --edition B --profile Y` | 两个组合的差异（包集合、生成物 diff 摘要） | 矩阵报告 |
| `zk why <package>` | 哪个角色/依赖把它拉进来 | 依赖排查 |
| `zk lock --profile X` | 更新 `zk.lock`（解析版本/源/哈希） | 发布 |
| `zk sbom --profile X` | 输出 SBOM（cargo + npm 汇总） | 发布合规 |
| `zk doctor` | 环境/工具链/网络/锁文件自检 | 新人与 CI |

解析算法（`compose`）：

```
1 读 edition → profile（含 base 链）→ OEM 叠层 → 角色映射与设置
2 建包索引：packages/*/*/zk.toml + vendor/*/zk.toml + OEM 子仓（可选）+ 外部源
3 校验：id 唯一、版本 semver、契约 major 匹配、角色基数、平台支持、OEM 品牌完整性
4 依赖图：包 requires 角色 → 找提供该角色的包 → 拓扑排序；环 = 错误
5 契约唯一性（fail closed）：cargo metadata 断言每个契约 crate 只有**一个 package id**
  （name+version+source 三元组），且等于根 [patch] 指定的那一份；
  拿不到 cargo metadata 直接报错，不降级（降级会漏掉同名副本，见 research/review-plan.md §6）
6 生成：apps/desktop/src/generated + apps/server/src/generated + apps/admin/web/src/generated
        + shared/contracts/*（zk api 生成）+ shared/ts/*（ts-rs 生成）+ deploy 生成段 + docs 索引
7 写/校验 zk.lock（含 edition/profile/oem 三层输入哈希）
```

**解析器不得依赖"工作区能编译"**：POC 踩过——`zk-composed` 被删后 `cargo metadata` 因
workspace member 缺 manifest 直接失败，形成鸡生蛋。所以包索引以 `zk.toml` 为唯一输入，
`cargo metadata` 只作为**契约单实例的交叉校验**；声称为契约唯一性把关的命令**拿不到 metadata 即失败，禁止降级**（`24` §8）。

## 5. 生成物（glue）：替换真正发生的地方

`zk compose` 产出全部**确定性**、**带 `@generated` 头**的文件；其中 `default` profile 的产物入库
（§5.5）。业务代码只依赖生成物，
不依赖具体包名——这是"换包零改源码"的物理实现。

### 5.1 Rust 粘合 crate `crates/zk-composed`

```rust
// @generated by zk compose --profile default -- do not edit
use zk_contract_package::{PackageDescriptor, Installed};
use zk_contract_kernel::Registrar;

pub const PACKAGES: &[PackageDescriptor] = &[ /* id/version/roles/kind/required */ ];

/// 每个包产出一个 cordis Plugin 适配器；ctx 只由 Registrar 持有，
/// 包不会直接拿到 Context（否则资源不归 fiber 所有，无法卸载/回滚）。
pub fn plugins() -> Vec<Installed> {
    use zk_tool_fs as pkg_tool_fs;          // 角色别名：换包只改这一行
    vec![
        pkg_tool_fs::install(),             // -> Installed { id, plugin: Arc<dyn Plugin>, required }
        // ...按 [requires] 拓扑序
    ]
}

pub async fn register_all(reg: &mut dyn Registrar) -> Result<StartupReport, StartupError> {
    let mut report = StartupReport::default();
    for inst in plugins() {
        let fiber = reg.install(inst.plugin, reg.config_for(inst.id)?)?;   // 内部= ctx.plugin(...)
        fiber.await_settled().await?;                                      // 关键：看状态，不看 Result
        match fiber.state() {
            FiberState::Active => report.active.push(inst.id),
            _ if inst.required => return Err(StartupError::PackageFailed { id: inst.id, detail: fiber.error_detail() }),
            _ => report.degraded.push((inst.id, fiber.error_detail())),
        }
    }
    Ok(report)
}
```

关键点：

- **注册必须经过 cordis 的 fiber**。`ctx.plugin(plugin, cfg)` 返回 `Ok(fiber)` 并不代表激活成功
  （内核实测：apply 失败仍返回 `Ok`），宿主必须 `await_settled()` 或检查 `state()`。
- **每个包的 effect/服务都归自己的 fiber**：卸载/失败时由内核逆序撤销。因此生成器产出的不是
  "调包里的 register(ctx)"，而是"每个包一个 Plugin 适配器"。
- **没有"整体回滚"**：内核只保证**单 fiber** 激活事务自回滚，多 key 原子替换做不到。启动失败策略是：
  `required=true` 的包失败 → 由 bootstrap `dispose_all()` 后进入诊断页并退出；`required=false` 的包失败
  → 记录 degraded，依赖它的角色在运行期显示为不可用（`inject` 门控会让依赖方不激活）。
- 产品二进制只链接被选中的包（`cargo build -p desktop`）；`cargo test --workspace` 仍编译所有包，
  这正是 T1/T2 分级存在的原因。
- `PACKAGES` 是运行期自省的唯一权威（诊断面板、`zk explain`、崩溃报告）；**包 id 由宿主维护**，
  cordis 的 `PluginKey`（Named/TypeId）不能当包身份（TypeId 跨构建不稳定）。
- 外部包的依赖形态由 `zk.lock` 决定：vendor/path → `{ path }`；git → `{ git, rev }`；
  registry → `{ version = "=x.y.z" }`。生成器只写解析结果。
- **平台入口也走同一机制**：`apps/desktop/Cargo.toml` 由 `zk compose` 生成（只依赖被选中的
  `platform.shell` 包），`main.rs` 固定为 `fn main() { zk_bootstrap::run() }`，不含任何具体平台 crate 名。

### 5.1b 服务端粘合 `apps/server/src/generated/`

```rust
// @generated by zk compose --edition enterprise -- do not edit
use zk_contract_server::ServerPackage;

pub const PACKAGES: &[PackageDescriptor] = &[ /* ... */ ];

pub fn packages() -> Vec<Box<dyn ServerPackage>> {
    use zk_srv_auth_local as pkg_auth;
    use zk_srv_gateway as pkg_gateway;
    vec![pkg_auth::package(), pkg_gateway::package() /* 拓扑序 */]
}

pub fn migrations() -> &'static [Migration] { /* 按包顺序汇总；哈希在 migrations.lock 校验 */ }
```

- 路由由 manifest `[[routes]]` 静态声明，`zk api` 生成 catalog/权限矩阵；运行期 `Register()` 只校验一致性（`24` §4）。
- 后台作业由生成器汇总，启动时注册到调度器。
- 服务端 glue 同样只有被选中的包；`edition=personal` 时这个文件根本不会生成。

### 5.2 UI 注册表 `apps/desktop/web/src/generated/`

```ts
// @generated by zk compose --profile default -- do not edit
import { components as c0 } from "@zeokeel/ui-chat";
import { components as c1 } from "@zeokeel/ui-settings-general";

export const registry: Record<string, ComponentType> = { ...c0, ...c1 };
```

由 vite `virtual:@zeokeel/registry` 暴露给 shell。shell 只认识 `componentId` 与 `slotId`，
不认识包名（详见 `16-ui-packages.md`）。替换 UI 包 = 改 profile 里对应角色 + 重新 compose。

### 5.3 契约产物 `shared/contracts/`

- `ipc-commands.json`：所有包声明的命令 + 参数/返回 JSON Schema（前端类型与文档的真源）。
- `ui-contributions.json`：**静态**贡献清单（来自各包 `zk.toml` 的 `[[ui.contributions]]`），
  含 slot / componentId / order / props schema 引用；构建期可与 web 侧组件表做闭包校验。
- `ui-slots.json`：槽位覆盖报告（哪些槽位被谁贡献）。
- `roles.json`：角色 → 包 → 契约 → 基数。
- `packages.json`：包清单（版本、kind、facet、稳定级别）。

### 5.4 TS 类型

`ts-rs` 从契约 DTO 导出 `.ts`（`cargo test -p zk-contract-* export_bindings`），CI 里
`git diff --exit-code` 防漂移。工具参数 schema 用 `schemars`（本来就是 JSON Schema，无需转换）。

### 5.5 生成目录、锁文件与 profile 的关系（重要，与 ADR-015 配套）

**事实（POC 实测）**：生成物 `crates/zk-composed/Cargo.toml` 的依赖边随 profile 变化，而 Cargo.lock
记录每个包的依赖列表，所以**一份 Cargo.lock 不可能对所有 profile 都 `--locked`**：

```
$ python3 resolve.py default  && cargo build --locked   # ok
$ python3 resolve.py hardened && cargo build --locked   # exit 101: cannot update the lock file
$ python3 resolve.py hardened && cargo build --offline  # ok，但 Cargo.lock 被重写
```

因此规则：

| 项 | 策略 |
| --- | --- |
| 生成物入库 | **只有 `default` profile**；头部带 `@profile` 标记，CI 断言提交的是 default |
| Cargo.lock | 入库的是 **default** 的解析结果；`default` job 用 `--locked` |
| 其他 profile 的构建 | CI 里 `zk compose --profile X` → `cargo build --offline`（锁文件在 CI 工作区重写，不提交）→ `zk verify-lock --profile X` 断言解析出的包集合/版本与 `zk.lock` 一致 |
| 离线来源 | `zk vendor` 生成**全 profile 并集**的 vendor 目录 + `[source] replace-with`，保证 `--offline` 可解析 |
| `pnpm-lock.yaml` | `apps/desktop/web/package.json` 的依赖生成成**候选包并集**（不是仅当前 profile），锁文件因此稳定；profile 只改变 registry 的 import |
| UI 生成物位置 | `apps/desktop/web/src/generated/`（**不是**独立 pnpm workspace 成员，避免 importer 列表随 profile 变化） |

为什么不用"每 profile 一个临时 workspace + 独立 target-dir"：workspace 继承（`workspace = true`）
会失效，且冷构建成本乘以 profile 数。并集 + 变体重解析是成本更低的正确取舍。

### 5.6 文档索引

`docs/generated/PACKAGE-INDEX.md`（所有包）、`PROFILE-MATRIX.md`（profile × 角色 × 包）、
`CONTRACT-INDEX.md`。数字一律生成，禁止手写——先例最大的返工来源就是手写计数漂移。

## 6. CI：把"可替换"变成可执行断言

### 6.1 流水线

```yaml
stages: validate → compose → build → test → matrix → package

jobs:
  manifests:        zk validate                       # 含契约唯一性（按 package id/source，fail closed）
  drift:            zk compose --profile default --check
                    gen docs --check / ts-rs bindings --check
  rust:             cargo nextest run --workspace --locked   # default 锁
  frontend:         pnpm -r typecheck && pnpm -r test:unit && pnpm build
  contracts:        cargo test -p zk-contract-testkit    # 每个实现的符合性套件
  matrix-os:        strategy: matrix.os = [ubuntu, macos, windows]   # M0 起就跑 cargo check
  matrix:           strategy: matrix.profile = [default, mock, minimal, hardened, oem-demo]
                    per profile: compose → cargo build --offline → zk verify-lock
                                 → verify-dist → Playwright(e2e) → 真应用冒烟
  swap:             strategy: matrix.swap = 生成的换包列表（见 6.2）
  external:         vendor/git 源 fixture 构建 + 负向测试（M0 只做 vendor 形态，见 6.5）
  negatives:        缺角色 / 重复 singleton / 契约两个 package id / 生成物手改 均应失败
  supply-chain:     cargo-deny + cargo-audit + pnpm audit + SBOM + 可复现构建校验
```

### 6.2 换包矩阵（swap matrix）——"实现类包都能被替换"的证明

算法：

```
index = zk validate 产出的角色→实现集合
for role in index.roles where role.swap_required:      # zk.toml 声明；本期 8 个角色
  for impl in index.providers(role) - profile[role]:
     profile' = profile.default with [packages][role] = impl
     jobs += build(profile') + smoke(profile')         # smoke = e2e 最小集 + 启动自检
```

成本控制（先例实测：一次客户端构建 ≈10 分钟，32 个缺席变体 ≈5 小时——这是"CI 可替换"落不了地的
真实原因，必须先解决再谈矩阵）：

测试分层 T0–T5、成本与 PR 子集策略见 [`11` §1](11-testing-and-evidence.md#1-分层)；本节只保留组合结论：T0 不编译也能证明组合正确（秒级），PR 只跑受影响子集（`zk diff`）。

- PR 只跑受影响子集：改了包 P → 跑"P 参与的每个角色"的换包用例（`zk diff` 可算）。
- 变体构建用**增量编译**：先构建 default，再仅重编 `zk-composed` + 受影响包（T3 的时间主要花在重链接）。
- 第一个 sprint 必须实测一次 T0–T3 基线，数字进看板；预算不是愿望，是门禁。
- 每个 profile 的冒烟集固定：启动自检（所有 required 服务就位）+ 一轮 mock LLM 对话
  （流式 + 工具调用）+ 设置打开 + 会话落库/重开。
- 超预算等价于机制失败，要优化（缓存/并行/子集），而不是接受。

### 6.3 生成物 diff 的最小性（防隐性耦合）

对换包前后的生成物做 diff，断言**只有被替换的那几行变化**：

```
expected: crates/zk-composed/{Cargo.toml,src/lib.rs} 中 role 对应行
unexpected: 其他包出现在 diff 中 → 说明有包越界依赖了具体实现，报警
```

这条比"能编译"更严格：它能发现"换 A 却意外把 B 也换进来了"这类组合泄漏。

### 6.4 负向测试（必须失败才算通过）

| 用例 | 期望 |
| --- | --- |
| profile 缺 required 角色 | `zk validate` 失败，指出缺哪个角色、被谁 require |
| 同一 singleton 角色两个实现 | 失败，列出两个包 |
| 提供者契约 major 与 requires 不匹配 | 失败 |
| 契约 crate 出现两个 package id（同名不同 path / 同名不同 git+rev / 改名副本） | 失败，指出两个 id；**不允许降级为警告**（POC 实测三种都能骗过"按版本去重"） |
| 第三方包自带私有契约副本 | `zk validate` 失败（"契约身份不符"），而不是留给 `cargo build` 的 E0599 |
| 手改生成物 | `--check` 失败 |
| UI 贡献声明的 componentId 在 web 侧不存在 | 构建失败（`required ⊆ provided`） |

### 6.5 第三方包接入（外部供应商/OEM）——必须先解决契约身份

**问题（评审实测）**：第三方包要依赖"同一份契约 crate"，否则 trait 身份不同 → 编译失败。而当前契约
既没发布到 crates.io，也没有固定 git rev；POC 里所谓"外部包"其实在仓库内（`experiments/.../external/`），
只是不在 Cargo workspace 里——**形状对，身份不对**。

因此第三方接入的正确设计是：

1. **契约发布 + 单实例强制**：`zk-contract-*` 以 `1.x` 发布到 crates.io（或内网 registry）；根
   `Cargo.toml` 由 `zk compose` 维护一段 `[patch.crates-io]`，把每个契约 crate 重定向到工作区内的
   唯一实例。外部包无论从 crates.io、git 还是 vendor 依赖契约，最终都链接**同一个** crate 实例。
2. **来源三选一**，优先级：`vendor/<name>/`（可审计、可离线）→ git（`zk.lock` 钉 commit）→
   registry（需公开契约版本）。
3. **接入门禁**（全部自动化）：`zk validate` → 契约身份唯一 → conformance 套件 → 换包矩阵 T2/T3 →
   许可证/供应链审计 → 生成物 diff 最小性。
4. **M0 只承诺 vendor 形态**：真正的"仓库外 git 源"需要第 1 步落地。M0 安排 **2 天 spike** 验证
   "发布 + patch 单实例"；若不可行，退回"契约以 git rev 提供，外部包必须用同一 rev"，并把结论写入
   ADR-016。

**信任边界是构建输入**（源码 + 锁定依赖 + 评审），不是运行时 manifest 标记；也不承诺插件沙箱。

## 7. 与 DSH 的对照

DSH 的一句话本质（`research/dsh.md` 一手证据）：**应用是数据，不是代码。** profile 的 `cordis.yml`
恒为空 `[]`，真正的插件树由 `package.json` 的 `dsh.profile.bundles` → 各 bundle 自带的 patch 文件 →
profile 的 `cordis.patch.yml` → `$DSH_HOME/cordis.patch.yml` → `--patch` overlay 逐层对同一份 entry
列表做 **id 定向 patch / insert** 合成。本机合成结果是 200 个顶层 entry / 194 个包 / 33 个 disabled。

| 机制 | DSH（运行时组合） | ZeoKeel（构建期组合） | 说明 |
| --- | --- | --- | --- |
| 组合单位 | npm 包（本机 305 个） | Cargo crate + 可选 npm facet | 数量级预期 45–70 |
| 组合描述 | bundle 列表 + 多层 patch YAML | `profiles/*.toml`（单链 base 继承） | 语义相近，时机不同 |
| 装配 | Node ESM resolver 按包名 `import` | `zk compose` 生成 `register_all` | **最不可移植的一层** |
| 换实现 | `disabled: true` 旧行 + `insert` 新行（或换 bundle） | 改 profile 的角色映射 + 重建 | ADR-001 |
| seam | 13 个 `Abstract … seam` 包只定服务，实现在独立包 | `zk-contract-*` crate 只定 trait/DTO | 同构 |
| 服务注册 | 同一 seam 内按 provider 名注册、互斥 | 组合期校验 singleton 基数 | 我们把互斥提前到构建期 |
| 前端装配 | `dsh.client` 清单 + boot graph + 懒加载 CJS 表 | 构建期静态注册表 + 虚拟模块 | 换包 = 重新 compose + build |
| 版本策略 | lockstep（294/305 包同为 `0.2.1-rc.2`，peer 精确 pin） | Cargo semver + 契约 major + `zk.lock` | Rust 有语义版本空间 |
| 失败时机 | 运行时 preflight / 跳过 bundle | 组合期 + 编译期 | 更强的静态保证 |
| 单二进制 | 否（Node + 包目录） | 是 | 产品差异 |

两个容易记错的细节：

1. **DSH 的替换不在依赖注入层**：patch 里的 `name` 是断言，改名会被 skip 并告警；必须"禁旧行 +
   插新行"。所以它的可替换性来自**组合层的 entry 操作**，不是 DI 容器换实现。
2. **一个包里可以带 patch**（bundle 即"一层能力 + 它的默认 entry"），这是它能把 305 个包组织成
   可维护组合的关键。我们在 `zk.toml` 里用 `[provides]`/`[requires]` 表达同一件事，但生成成代码。

值得照搬的是**组织方式**（包边界、seam 命名、profile 形态、多实现并存、前端包清单）；
不值得照搬的是**运行时装配**——那是 Node 生态的红利，Rust 侧用代码生成换回来。

## 8. 暂不纳入范围的"运行时切换"（M5 之后再说）

内核研究确认 `Registry::replace` / `Fiber::replace_plugin` 可以在**已链接的多个实现之间**做语义安全的
运行时替换（保持 fiber 身份、失败回滚）。这在产品上有价值（安全模式切换、多 provider 热切）。

但**本期不做**，原因：

- `[alternates]` 会让 profile 多出一套组合语义，与"单一组合输入"冲突（评审 S6）；
- 同时链接两个实现会削弱"没选中的包不进二进制"的体积收益；
- 它容易被误解成"运行时换包"，而真正的运行时换包在静态链接下不存在。

保留角色命名空间 `tool.wasm.*` / `host.wasm` 作为未来扩展点（ADR-013），等 M4 产品稳定后再评估。
