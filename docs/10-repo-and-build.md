# 10 · 仓库布局与构建

> 目标：**加一个包/客户不用改根配置**；**生成物只有 `zk` 能写**；**客户端与服务端共享契约、锁与 CI**。

## 1. 顶层布局

```text
ZeoKeel/
├─ Cargo.toml                 # 单一 workspace：members = ["packages/*/*","apps/*","tools/*"]
│                             #   exclude = ["oem/*","vendor/*","fixtures/*","experiments/*"]
├─ Cargo.lock
├─ rust-toolchain.toml         # ≥1.97.1（rig 0.45 MSRV；edition 2024）
├─ pnpm-workspace.yaml         # packages: ["apps/*/web","apps/admin/web","packages/*/*/web"]
├─ package.json                # 根脚本
├─ zk.toml                     # zk 配置：包扫描根、契约登记、生成路径、OEM 来源
├─ zk.lock                     # 组合锁（edition/profile × 角色→包/版本/源/哈希）
├─ editions/                   # enterprise.toml / personal.toml（版本包含哪些包）
├─ brands/                     # official 渠道品牌资产（logo/文案/安装器文案；OEM 子仓同构）
├─ profiles/                   # base/default/mock/minimal/hardened/ci-matrix/...
├─ packages/
│  ├─ contracts/               # zk-contract-*（trait/DTO/API 契约 + conformance kit）
│  ├─ runtime/                 # zk-runtime-*（cordis 宿主、config、ipc、http、db、migrate、jobs、obs）
│  ├─ client/                  # zk-agent-*、zk-tool-*、zk-llm-*、zk-client-*、UI 声明 crate
│  ├─ server/                  # zk-srv-* 域包 + admin-* 管理台页面包（Rust 声明 crate + web/ facet）
│  └─ ui/                      # @zeokeel/ui-*/@zeokeel/admin-*：Rust 声明 crate + web/ facet
├─ apps/
│  ├─ desktop/                 # Tauri 客户端（Cargo.toml 由 zk 生成）
│  ├─ server/                  # axum 服务端（Cargo.toml 由 zk 生成）
│  └─ admin/                   # 管理台 SPA（vite 根；产物内嵌服务端）
├─ oem/                        # 我们自建 OEM 索引仓的 submodule 挂载（私有；CI 用 deploy key；参考仓不直接使用）
├─ deploy/
│  ├─ docker/                  # Dockerfile（多阶段）、compose、Caddyfile、入口脚本
│  └─ portal/                  # 员工下载门户（服务端静态资源，随渠道品牌）
├─ shared/                     # 生成：api-catalog.json、openapi.json、ts 类型、permissions、schema.md
├─ scripts/                    # bootstrap/dev/test/ci/release/evidence
├─ configs/                   # 严格检查参考配置（scaffold 时复制；见 docs/18）
├─ tools/zk/                   # 组合器 CLI（workspace 成员）
├─ fixtures/                   # 负向测试、第三方包 fixture（不进 workspace）
├─ vendor/                     # 外部包实体（path 依赖）
├─ experiments/composition-poc/
├─ research/
└─ docs/
```

**为什么客户端与服务端共用一个 Cargo workspace**：

- 契约 crate 必须**单实例**（trait 身份）；跨 workspace 的 path 依赖会破坏 `workspace = true` 继承。
- 一份 `Cargo.lock` 覆盖两端，CI 只维护一个锁。
- 共享 `zk-contract-*` / `zk-runtime-*` / `zk-ui-*` 无需发布。
- 代价：Tauri 与 axum 依赖树合并、锁更大（预计 400–600 包），靠 `cargo check -p`、sccache 与分层 CI 控制。

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

有 UI facet 的包加 `web/`（package.json + src）；有后台作业的包在 `zk.toml` 声明 `[jobs]`。

## 3. editions 与 profiles

> edition 包集合、`base_edition`、`exclude`、profile/channel 叠加的权威示例见 [`03` §3](03-composition-and-ci.md#3-组合输入edition--profile--channel-叠层) 与 [`24` §1](24-composition-and-runtime-spec.md#1-组合层与优先级冻结)。本节只讲目录与 ownership。


profile 继承 edition，再按环境覆盖（`profiles/base.toml` → `default.toml` → `mock.toml`）。
OEM 在 edition 之上再叠一层 `oem/<customer>/oem.toml`（见 09）。

## 4. 生成物（只有 `zk` 能写）

| 路径 | 生成者 | 说明 |
| --- | --- | --- |
| `apps/desktop/src/generated/`、`apps/server/src/generated/` | `zk compose` | 两端 glue：注册包、路由表、UI registry、迁移清单 |
| `shared/contracts/{api-catalog,http-routes,openapi,permissions}.json` | `zk api`（见 `24` §3–§4） | 全量端点 + 权限点 + 请求体上限 |
| `shared/ts/*.ts` | ts-rs（唯一） | DTO/事件/API 类型 |
| `deploy/docker/Dockerfile` 生成段 | `zk compose` | 渠道/版本/客户端安装包清单 |
| `docs/generated/*.md` | `zk docs` | 包索引、端点矩阵、权限矩阵、schema |
| `migrations.lock` | `zk db lock` | 所有迁移内容哈希 |

规则：**default 产物入库**；其他 profile/渠道在 CI 内生成不提交；头部带 `@generated`/`@profile`；
`--check` 防手改；生成顺序确定（按包 id 排序、无时间戳）。

## 5. 工具链与依赖钉死

- Rust `1.97.1+`（rig 0.45 MSRV / edition 2024）；`zeokeel-cordis = "=0.6.x"`。
- `rig-core = "=0.45.0"` 只出现在 `zk-llm-rig`；`rig-agent`/`rig-memory` 不进 M1。
- 服务端：`axum`、`tokio`、`sqlx`（Postgres + migrate）、`wasmtime`（WASM 应用）、`tower`、`tracing`。
- 前端：React 19 + Vite + Tailwind 4；`pnpm-lock.yaml` 入库；依赖用候选包并集（ADR-015）。
- `serde_json` 的 `preserve_order`/`float_roundtrip` 全局 feature 需统一决策（rig 0.45 副作用）。
- 依赖门禁：`cargo-deny` + `cargo-audit` + `pnpm audit`；第三方许可清单随产物。

## 6. 锁文件

| 锁 | 内容 | 策略 |
| --- | --- | --- |
| `zk.lock` | 每个 edition/profile/渠道的组合解析结果 | 入库；变更即组合变更 |
| `Cargo.lock` | default 组合的解析结果（两端同锁） | 入库；default job `--locked` |
| `pnpm-lock.yaml` | UI 依赖并集 | 入库；default `--frozen-lockfile` |
| `migrations.lock` | 迁移内容哈希 | 入库；启动校验 |

变体/OEM 渠道在 CI 内 `--offline` 重解析 + `zk verify-lock`（ADR-015），产物不提交。

## 7. 开发循环

```bash
bash scripts/bootstrap.sh                 # 系统库 + pnpm + cargo check + 生成
bash scripts/dev-desktop.sh               # Tauri dev + vite HMR + 变更自动 zk compose
bash scripts/dev-server.sh                # 服务端 + 临时 PG（compose 或 testcontainers）
bash scripts/dev-all.sh                   # 服务端 + 客户端联调（企业版登录/网关）
bash scripts/check.sh                     # validate + compose --check + clippy + typecheck
bash scripts/test.sh [--edition E] [--profile P]
```

Rust 改动无热替换（重编译+重启）；前端 vite HMR；改 `zk.toml`/profile/OEM 必须重跑 `zk compose`。

## 8. 离线与跨平台

- `.gitattributes`：`* text=auto eol=lf`（Windows checkout 不影响 `--check` 字节比对）。
- 离线构建：`zk vendor` 生成全 profile 并集 vendor；`pnpm fetch` 缓存；Tauri bundler（NSIS/WiX）、
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
