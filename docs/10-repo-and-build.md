# 10 · 仓库布局与构建

> 目标：**加一个包/客户不用改根配置**；**生成物只有 `zk` 能写**；**客户端与服务端共享契约、锁与 CI**。

## 1. 顶层布局

```text
ZeoKeel/
├─ Cargo.toml                 # 单一 workspace：members = ["packages/*/*","apps/*","crates/*","tools/*"]
│                             #   exclude = ["oem/*","vendor/*","fixtures/*","experiments/*"]
├─ Cargo.lock
├─ rust-toolchain.toml         # 1.99.1（≥1.97.1 rig MSRV；本机/CI 同版本）
├─ pnpm-workspace.yaml         # packages: ["apps/*/web","packages/*/*/web"]
├─ package.json                # 根脚本 + packageManager 固定 pnpm
├─ zk.toml                     # 组合根清单：scan/roles/contracts/paths（见 24 §2.1）
├─ zk.lock                     # 组合锁（edition/profile/channel × 角色→包/版本/源/哈希）
├─ editions/                   # enterprise.toml / personal.toml / personal-only.toml
├─ brands/                     # official 渠道品牌资产（logo/文案/安装器文案；OEM 子仓同构）
├─ profiles/                   # base/default/mock/minimal/hardened/ci-matrix/...
├─ crates/                     # 非包 crate：zk-composed（生成 glue）、zk-bootstrap（引导，ADR-009）、zk-kit-*（共享实现库，`25`）
├─ packages/
│  ├─ contracts/               # zk-contract-*（trait/DTO/API 契约 + conformance kit）
│  ├─ runtime/                 # zk-runtime-*（cordis 宿主、config、ipc、http、db、migrate、jobs、obs）
│  ├─ client/                  # zk-agent-*、zk-tool-*、zk-llm-*、zk-client-*、UI 声明 crate
│  ├─ server/                  # zk-srv-* 域包 + admin-* 管理台页面/壳包（Rust 声明 crate + web/ facet）
│  └─ ui/                      # @zeokeel/ui-*：Rust 声明 crate + web/ facet（不含 admin）；ui-kit = 共享组件（角色 ui.kit）
├─ apps/
│  ├─ desktop/                 # Tauri 客户端：web/（前端）+ src/（Rust 入口；Cargo.toml 由 zk 生成）
│  ├─ server/                  # axum 服务端（Cargo.toml 由 zk 生成）
│  └─ admin/web/               # 管理台 SPA（vite 根；产物内嵌服务端）
├─ oem/                        # 我们自建 OEM 索引仓的 submodule 挂载（私有；CI 用 deploy key；参考仓不直接使用）
├─ deploy/
│  ├─ docker/                  # Dockerfile（多阶段）、compose、Caddyfile、入口脚本
│  └─ portal/                  # 员工下载门户（服务端静态资源，随渠道品牌）
├─ shared/                     # 生成：contracts/（api-catalog 等）、ts/、schema/（JSON Schema）
├─ scripts/                    # bootstrap/dev/test/ci/release/evidence
├─ configs/                   # 严格检查参考配置（scaffold 时复制；见 docs/18）
├─ tools/zk/                   # 组合器 CLI（workspace 成员）
├─ specs/                      # 变更规格（changes/templates/archive）
├─ .agents/skills/             # 项目 Agent Skills
├─ fixtures/                   # 负向测试、第三方包 fixture（不进 workspace）
├─ vendor/                     # 外部包实体（path 依赖）
├─ experiments/composition-poc/
├─ experiments/wasm-poc/
├─ research/
└─ docs/                       # 规划文档 + generated/（zk docs 生成）
```

- **实现状态（task-13）**：`crates/zk-bootstrap` 已存在（ADR-009 的非包引导代码）：创建并持有
  `zeokeel-cordis` kernel root、注册 `zk-contract-kernel` 的三个宿主服务、以 kernel effect 监督后台任务、
  统一 `dispose_all()` 关闭；`apps/desktop` 与 `apps/server` 的 `main.rs` 都经它启动/关闭。
  内核版本精确钉在 `Cargo.toml` 的 `zeokeel-cordis = "=0.6.0"`（见 §5）。

**为什么客户端与服务端共用一个 Cargo workspace**：

- 契约 crate 必须**单实例**（trait 身份）；跨 workspace 的 path 依赖会破坏 `workspace = true` 继承。
- 一份 `Cargo.lock` 覆盖两端，CI 只维护一个锁。
- 共享 `zk-contract-*` / `zk-runtime-*` / `zk-ui-*` 无需发布。
- 代价：Tauri 与 axum 依赖树合并、锁更大（量级估算 400–600 包；正式数字由 `zk plan` 生成），靠 `cargo check -p`、sccache 与分层 CI 控制。

## 2. 一个包的目录形态

```text
packages/server/wallet/
├─ zk.toml                     # 唯一真源：id/version/kind/roles/requires/facets
├─ Cargo.toml                   # crate zk-srv-wallet
├─ src/lib.rs                   # pub fn package() -> impl ServerPackage
├─ migrations/<global-seq>_<name>.sql   # 全局序号由 zk db lock 分配（24 §5）
├─ schema/config.json           # 系统配置项 schema（schemars 生成）
├─ tests/conformance.rs         # 服务端契约符合性
└─ README.md                    # 角色、依赖、替换注意
```

有 UI facet 的包加 `web/`（package.json + src）；有后台作业的包在 `zk.toml` 声明 `[[jobs]]`（`24` §2）。

**共享实现库不是包**：Rust 的 `crates/zk-kit-*` 无 `zk.toml`/无角色、只依赖契约与其它 kit；TS 的 `packages/ui/ui-kit` 是包（角色 `ui.kit`）。业务包不得互相 import；规则见 `25` §2。

## 3. editions 与 profiles

> edition 包集合、`base_edition`、`exclude`、profile/channel 叠加的权威示例见 [`03` §3](03-composition-and-ci.md#3-组合输入edition--profile--channel-叠层) 与 [`24` §1](24-composition-and-runtime-spec.md#1-组合层与优先级冻结)。本节只讲目录与 ownership。


profile 在 edition 解析结果之上做环境覆盖（`profiles/base.toml` → `default.toml` → `mock.toml`）；channel（含 OEM）在 profile 之上叠加（`24` §1.1）。

## 4. 生成物（只有 `zk` 能写）

**唯一“产物 → writer → 路径 → `--check`”表见 [`24` §3](24-composition-and-runtime-spec.md#3-生成物所有权一路径一-writer)。**
本节只强调与本仓库布局相关的事实：

- Rust glue 在 `crates/zk-composed/`；UI registry 在 `apps/desktop/web/src/generated/`；服务端 glue 在
  `apps/server/src/generated/`；管理台在 `apps/admin/web/src/generated/`。
- 契约产物在 `shared/contracts/`（计划 `zk api` 与 `zk compose` 分工见 `24` §3；`zk api` 未实现）；Schema 由 schemars（`export_schema`）导出，TS 由 `scripts/schema-ts.py` 从 schema 生成。
- 文档索引在 `docs/generated/`，writer 是 `zk docs`。

规则：**基线 `personal × default × official` 的产物入库**；其它组合在 CI 内生成不提交；头部
`@generated` + `@edition/@profile/@channel`（`24` §1.2/§3）；`--check` 防手改；生成顺序确定（按包 id 排序、无时间戳）。

## 5. 工具链与依赖钉死

- Rust `1.99.1`（`rust-toolchain.toml` 固定；≥1.97.1 是 rig 0.45 MSRV / edition 2024；本机与 CI 同版本）。
- `zeokeel-cordis` 锁到具体 patch（`=0.6.z`，z 在 scaffold 时按发布版本写入 `Cargo.lock`，不用 `0.6.x` 这类范围；具体版本在 T2 确认）。
- `rig-core = "=0.45.0"` 只出现在 `zk-llm-rig`；`rig-agent`/`rig-memory` 不进 M1。
- 服务端：`axum`、`tokio`、`sqlx`（Postgres + migrate）、`wasmtime`（WASM 应用；M4 定版，候选 LTS 48/50，原型用 50，`15` §C T11）、`tower`、`tracing`。
- 前端：React 19 + Vite + Tailwind 4；Node 22 LTS、pnpm 10+（`packageManager` 固定）；`pnpm-lock.yaml` 入库；依赖用候选包并集（ADR-015）。
- `serde_json` 的 `preserve_order`/`float_roundtrip` 全局 feature 在 M0 spec 定（`15` §C T15）。
- 依赖门禁：`cargo-deny` + `cargo-audit` + `pnpm audit`；第三方许可清单随产物。

## 6. 锁文件

| 锁 | 内容 | 策略 |
| --- | --- | --- |
| `zk.lock` | 每个 edition/profile/渠道的组合解析结果 | 入库；变更即组合变更 |
| `Cargo.lock` | default 组合的解析结果（两端同锁） | 入库；default job `--locked` |
| `pnpm-lock.yaml` | UI 依赖并集 | 入库；default `--frozen-lockfile` |
| `migrations.lock` | 迁移内容哈希 | 入库；启动校验 |

变体/OEM 渠道在 CI 内 `--offline` 重解析 + 组合锁校验（`zk verify-lock`，**未实现/backlog**），产物不提交。

## 7. 开发循环（scaffold 后）

```bash
bash scripts/bootstrap.sh                 # 系统库 + pnpm + cargo check + 生成
bash scripts/dev-desktop.sh               # Tauri dev + vite HMR + 变更自动 zk compose
bash scripts/dev-server.sh                # 服务端 + 临时 PG（compose 或 testcontainers）
# bash scripts/dev-all.sh                 # 未实现/backlog；企业版登录/网关联调改用 scripts/login-e2e.sh / admin-e2e.sh
bash scripts/check.sh                     # validate + compose --check + clippy + typecheck
bash scripts/test.sh [--edition E] [--profile P]
```

Rust 改动无热替换（重编译+重启）；前端 vite HMR；改 `zk.toml`/profile/OEM 必须重跑 `zk compose`。

## 8. 离线与跨平台

- `.gitattributes`：`* text=auto eol=lf`（Windows checkout 不影响 `--check` 字节比对）。
- 离线构建：`zk vendor`（未实现，backlog）生成全 profile 并集 vendor；`pnpm fetch` 缓存；Tauri bundler（NSIS/WiX）、
  linuxdeploy、Playwright 浏览器、webkit2gtk 等系统依赖**预烤 CI 镜像**。
- CI 从 M0 起三平台 `cargo check`；M1 起三平台安装包冒烟；M5 起 **macOS 公证**（Windows 不签名，ADR-033）。
- 交叉编译：Windows/Linux 用对应 runner 原生构建，macOS 用 macOS runner；不在 Linux 交叉编 mac。

## 9. 代码风格与门禁

- Rust：`clippy -D warnings`；**手写代码零 `unsafe`（产品 crate `forbid`）**；生成绑定/独立 `*_sys` crate 的例外见 `18` §4；
  第三方依赖内的 unsafe 用 `cargo-geiger`/`cargo-deny` 审计，不自己写 unsafe。
- 包四件套：`zk.toml` + 契约 + conformance 测试 + README；`zk validate` 强制。
- 服务端包额外要求：迁移哈希、路由权限声明、作业幂等说明。
- 文案不写死在代码里：i18n key + 词条包；OEM 覆盖走词条。
- 生成物禁止手改；数字/索引一律生成。


## M1 当前实际结构（与上文目标一致的最小集）

```text
tools/zk/  crates/zk-composed/  packages/{contracts,client,ui}/  apps/{desktop,server}/  editions/ profiles/ brands/ shared/ scripts/
```

- 生成物：`crates/zk-composed/`、`apps/*/Cargo.toml`、`apps/*/src/generated/`、`shared/contracts/{packages,roles,brand,ui-contributions}.json`、`docs/generated/*`、`apps/desktop/web/src/generated/{brand,registry}.ts`；只由 `zk compose` 写。
- UI 包 facet 在包内 `web/`；pnpm workspace 覆盖 `apps/*/web` 与 `packages/*/*/web`。
