# AGENTS.md

> 给 AI coding agent 的项目说明（"README for agents"），格式参考 [agents.md](https://agents.md)。
> **最近的 AGENTS.md 优先；用户对话中的明确指令优先于本文件。**
> 本文件是**约束**，不是介绍；与它冲突的实现一律拒绝，先改文档/ADR 再改代码。

## 1. Project overview

**ZeoKeel**：一切皆包的 Rust 企业级 Agent 平台。

- **客户端是主体**：Tauri 2 + React 19 + TypeScript 桌面客户端；本地 agent（`rig-core` 0.45）、工具、审批、会话、技能/连接器/应用客户端。
- **服务端是弱管控配套**：Rust（axum + PostgreSQL + cordis）只做认证、用户、钱包/网关计费、商店分发（技能/连接器/应用）、审计、管理台；**不执行 agent、不执行技能**。
- **内核**：`zeokeel-cordis` 0.6.x（纯 Rust、静态链接、无动态加载）。
- **组合方式**：构建期组合（edition/profile/channel → `zk` → 生成 glue → 编译）；**运行期不加载任何插件/代码**。
- **复用优先**：可复用实现只在契约/kit；能力索引由 `zk capability` 生成；禁止第二份实现。
- **双版本**：`personal`（纯客户端、自配 provider、无应用中心）是基线；`enterprise` = (个人版 − 个人专属包) + 企业管控包（服务端、登录、网关、商店客户端、管理台），通过 `base_edition = "personal"` + `exclude` 追加/覆盖，不复制客户端基线。
- **OEM**：私有索引仓 + 每客户 submodule，构建期注入渠道/品牌/文案；OEM 定制代码由我方开发并入库（D61）。
- **许可**：MIT（企业版/个人版核心）；OEM 产物按交付说明。

### 当前阶段（重要）

仓库已从 Plan 阶段进入 **M0 骨架 + M1 个人版客户端**：

- 已有：`Cargo.toml` 单 workspace、`zk` 组合器（validate/compose/plan/capability/find）、契约（package/kernel/tool/llm/agent/ipc/session/config/ui）、
  client 包（tool-fs/llm-mock/agent-loop/store-session-sqlite/store-settings-json/llm-openai-compatible）、workspace 契约/v2 迁移、
  `crates/zk-composed`（生成 glue）、Tauri 桌面 app + axum 服务端骨架、serve 载体、assistant-ui React 前端（UI 包化）、官方渠道 logo 构建期注入。
- 尚未完成：provider 运行时按设置选择/系统密钥库（M1b）、审批/工具扩展、右栏/命令面板完整版、
  M2 企业版服务端（**C3 服务端拆分完成**：`zk-kit-server` + auth/users/wallet/pricing/gateway/audit/settings/jobs 独立包，`zk-srv-http` 仅 health/ready/core migration（约 100 行）；各域自带 migrations，`apps/server` 为 composition root；管理台已拆 `admin-shell`/`admin-ui-kit`/`admin.page.*` 包，并由 `zk compose` 生成 `apps/admin/web/src/generated/registry.ts`；`ui-shell` 已承接 AppShell/FirstRun、`ui-commands` 已接入 ⌘K，desktop 薄壳只剩 transport/controller/reducer/i18n/theme/login gate（App 约 139 行）；开发原型移到 `research/ui-prototypes/`；P1 已有 srv.http/PostgreSQL 迁移、local 登录/EdDSA JWT/refresh/admin cookie+CSRF/users/roles、钱包账本/调账、价格/模型目录、网关准入/hold/SSE/结算、运维 jobs（hold 过期/pending/对账/月发放）；管理台 SPA（用户/钱包/价格/审计/用量）已可登录操作；`scripts/admin-e2e.sh` 真实 PG+Playwright 6 tests；deploy/ 已有 Dockerfile/compose/Caddy/portal/backup（本机无 Docker，未构建验证）；`zk-client-auth` 登录/refresh/me/logout + `@zeokeel/ui-login` 已接入 desktop：enterprise edition + 首配/登录门禁；`scripts/login-e2e.sh` 真实 server 通过；`zk-llm-gateway` 企业 provider 已接登录 token 注入（IPC `auth.configure`），enterprise 登录→网关聊天 e2e 通过；客户端按 expires_in 刷新并重注入 token）、M3–M6。
- 文档仍是规范真源；代码与文档冲突时先改文档/ADR 再改实现。

## 2. 最高约束（必须遵守）

| # | 约束 | 违反后果 |
| --- | --- | --- |
| C1 | **个人版优先**：先做个人版客户端（壳/UI/agent/工具/本地 provider）；**企业版 = 个人版 + 企业管控包**（登录/网关/商店/服务端/管理台），并排除个人版专属包（本地 provider 设置 UI、GitHub 更新器），不另造客户端基线；WASM 应用中心后置 | 拒绝合并 |
| C2 | **构建期组合**：包集合由 edition/profile/OEM 在构建时决定；运行期无加载器、无热插拔、无"解锁"代码 | 拒绝合并 |
| C3 | **一切皆包**：业务能力、UI、服务端域、管理台页面都是包；包必须四件套（`zk.toml` + 契约 + conformance 测试 + README） | 拒绝合并 |
| C4 | **契约是唯一稳定缝**：包之间只依赖 `zk-contract-*`；每个契约 crate 单实例；跨包禁止依赖具体实现类型 | 编译/CI 失败 |
| C5 | **`requires` 只写角色，不写包名** | 换包即失效 |
| C6 | **企业版弱管控**：不做工具白名单、不做网络限制、不做企业审批；服务端只控制身份/钱/商店/构建期 | 见 §11 禁止项 |
| C7 | **企业版 AI 只走网关**：只支持 OpenAI **Responses API**；客户端不得持有上游 key、不得填 base_url | 见 ADR-031 |
| C8 | **无部门/用户组**；角色只有超管/审计员/员工；内容可见性 = 全员/指定用户 | 见 ADR-032 |
| C9 | **TS/Rust 最严格检查**：`docs/18-strictness.md`；禁止 `any`/`as`/`unwrap`/`expect`/`panic`，禁止放宽全局 lint | CI 0 容忍 |
| C9b | **Rust 手写代码零 `unsafe`**：所有产品 crate（packages/apps/tools/crates）`#![forbid(unsafe_code)]`，禁止手写 `unsafe`/`#[allow(unsafe_code)]`；FFI/WASM 导入只能用**自动生成绑定**（`@generated`，单独审计）或独立 sys crate；第三方与生成代码内的 unsafe 用 `cargo-geiger`/`cargo-deny` 管理 | 拒绝合并 |
| C10 | **外部输入必须运行时校验**：Rust 生成 JSON Schema，前端 Ajv 校验；禁止类型断言代替校验（**M1 过渡例外见 ADR-059；T22 是 M2 前硬退出条件**） | CI/测试失败 |
| C11 | **生成物只能由生成器写**：`@generated` 文件禁止手改；`--check` 必须绿 | CI 失败 |
| C12 | **任何决策先文档后代码**：未在 `docs/` 落定的功能不实现；不确定就问用户 | 返工 |
| C13 | **更新通道固定**：个人版只走 GitHub Releases；企业版客户端只从登录服务端按 1:1 更新；企业版服务端必须手动导入镜像升级；禁止客户端直连公共更新面/跨渠道 | 见 ADR-041 |
| C14 | **渠道决定登录地址形态**：官方企业版也是渠道（`official`），`server_url` 默认空 → 首配页由客户填写并本地保存（设置可改，需重登）；OEM 渠道可设非空 → 登录页**不显示地址输入**（只账号/密码），地址不可编辑；个人版必须空且无登录 | 见 ADR-042/044 |
| C15 | **参考仓库只作参考**：`zeokeel/channels`、`ZeoKeel-Harness`、`ZeoKeel-Next` 等只可作事实/设计参考，**不得**作为我们的依赖、构建输入、索引仓或数据源；我们的 OEM 索引仓必须自有命名（demo 为 `zeokeel/demo-oem-index`） | 拒绝合并 |
| C16 | **无 spec 不开工**：代码/契约/数据/行为变更必须先有 `specs/changes/<id>/` 五件套；spec 经用户确认后改 `approved`；完成必须填 `acceptance.md` 并归档 | 拒绝合并 |
| C17 | **复用优先 / 单一实现**：可复用能力只允许一处实现（契约 / `crates/zk-kit-*` / `packages/ui/ui-kit`）；禁止新建 `utils/helpers/common` 或复制近似实现；新增可复用能力必须先查能力索引并登记 `@capability`；CI 重复/越界门禁（`docs/25`、ADR-057） | 拒绝合并 |
| C18 | **对话必须归属工作区**：无工作区不建会话；会话 `workspace_id` 创建后不可变（唯一例外：用户显式 `workspace.remove{reassign_to}`，需二次确认/审计）；agent cwd/工具默认根 = 工作区根；会话列表/消息按工作区隔离（归档为 post-M1；`docs/26`、ADR-058） | 拒绝合并 |
| C19 | **发现即文档，评审后施工**：发现有用方案（库/设计系统/DSH 模式/颜色/品牌规则等）先写入 `docs/`（或 ADR），由非实现者只读评审；实现者修复 P0/P1；同一评审者复评通过后，才可写 approved spec/代码；禁止"先写好看/好用代码再补文档"（`docs/11` §9、`docs/27` §5） | 拒绝合并 |

## 3. Architecture model（binding）

### 3.1 客户端

- Tauri 2 窗口 + 同源 HTTP 副载体（`serve`）用于 e2e/远程；WebView 加载 React UI。
- 本地 agent 循环（自有状态机）+ `rig-core` 只做模型调用；工具/审批/会话/检查点都在本地。
- provider：个人版本地（DeepSeek/OpenAI 兼容/Ollama）；M1 key 暂存本地 JSON（0600），M2 接系统密钥库；企业版 `llm.provider.default`。
- 前端组件是包：pnpm workspace + `[[ui.contributions]]` 静态声明 + 构建期生成 registry；shell 只认 componentId/slotId。
- UI 基线：assistant-ui primitives + Radix 原语 + Tailwind 4；流式 Markdown 用 `@assistant-ui/react-markdown`；长列表后置 TanStack Virtual。视觉/包组合/品牌见 `27`。
- 未登录（企业版）：`server_url` 非空（渠道预置，OEM 常见）→ 登录页只显示账号/密码；为空（官方企业版默认）→ **首配页（`ui.first-run`）** 显示地址输入并保存（设置可改，需重登），然后进登录页；登录页本身不显示地址输入。

### 3.2 服务端（弱管控）

- 单进程 axum + PostgreSQL（唯一库）；无 Redis/MQ/S3；管理台/门户/客户端安装包内嵌镜像。
- 认证：单一主源（local/OIDC/LDAP）+ 本地应急超管；JWT access + refresh；禁用后**每请求校验 `token_epoch` 立即失效**。
- 钱包：个人余额一本账；月发放默认全员、累加/覆盖两种模式；仅管理员充值；峰谷价；超额硬阻断；流式允许透支。
- 网关：只支持 `POST /v1/responses`；上游 key 只在服务端；未定价模型拒绝；结算与 usage 同事务。
- 商店：技能（DSH zip，16MB，客户端执行，不扫描）、连接器（MCP，允许 stdio，OAuth 回调在客户端）、WASM 应用（服务端 wasmtime 执行、每应用 SQLite、不可调 AI）。
- 审计 ≥180 天、用量 ≥12 个月，可配置但不得低于下限。
- 迁移不可变 + 内容哈希 + 只前向修复；路由唯一真源 + 权限点同处申报。

### 3.3 安全边界（写死）

- 服务端**不为员工行为兜底**；工具执行/命令/网络/文件在员工机器上，安全责任在客户终端与网络边界。
- 不承诺插件沙箱；WASM 应用是唯一沙箱（无 AI/网络/文件，仅宿主 DB/日志/资源函数）。
- 信任边界 = 构建输入（源码 + 锁文件 + 评审 + 可复现构建），不是运行时 manifest 标记。

## 4. Repository layout

### 当前（M1 进行中）

```text
ZeoKeel/
├─ AGENTS.md  README.md  Cargo.toml  zk.toml  pnpm-workspace.yaml
├─ docs/                # 规划（00–28）+ PLAN + README(SSOT)
├─ specs/               # 变更规格（changes/templates/archive）
├─ editions/ profiles/ brands/
├─ crates/zk-composed/  # 生成 glue
├─ packages/contracts/  # package/kernel/tool/llm/agent/ipc/session/config/ui
├─ packages/client/     # tool-fs/llm-mock/agent-loop/store-*/llm-openai-compatible
├─ packages/ui/         # ui-kit/ui-sessions/ui-chat/ui-settings（web facet）
├─ apps/desktop/        # Tauri shell + serve 载体 + React web
├─ apps/server/         # axum 骨架（M2 扩展）
├─ tools/zk/ scripts/ shared/ target/（不入库）
└─ research/ experiments/ configs/
```

### 目标（scaffold 后）

```text
ZeoKeel/
├─ Cargo.toml            # 单一 workspace
├─ pnpm-workspace.yaml
├─ zk.toml / zk.lock
├─ editions/  profiles/  brands/
├─ crates/               # 非包 crate：zk-composed（生成 glue）、zk-bootstrap（ADR-009）、zk-kit-*（共享实现库，25）
├─ packages/{contracts,runtime,client,server,ui}/
│                          # ui/ui-kit = 共享 UI 组件（角色 ui.kit）
├─ apps/{desktop,server,admin}/
├─ deploy/{docker,portal}/
├─ oem/                  # OEM 索引仓 submodule
├─ specs/               # 变更规格（changes/templates/archive）
├─ .agents/skills/      # 项目 Agent Skills
├─ tools/zk/
├─ shared/               # 生成的 contracts/ts/schema
├─ scripts/
└─ docs/  research/  experiments/  configs/
```

详细见 `docs/10`；组合/manifest/运行时以 `24` 为准；客户端交互 `19`、运维 `20`、安全 `21`、钱 `22`、OEM `23` 为补充规范。

## 5. Setup commands

### 【当前可用】工具链

```bash
# 工具链（本机已具备，CI 需同版本）
rustc --version          # 1.99.1；要求 >= 1.97.1（rig MSRV）
cargo --version
node --version           # >= 22（本机 26）
pnpm --version           # >= 10（本机 12）
PATH=/usr/local/go/bin:$PATH go version   # 1.27.x（仅 wasm-poc guest 需要）
rustup target add wasm32-wasip1

# 海外站点/依赖下载：必须走代理
export HTTPS_PROXY='socks5://<proxy>'; proxychains -q <cmd>

# GitHub（已授权，账号 zeokeel-dev，org zeokeel）
gh auth status
```

### 【scaffold 后】初始化

```bash
bash scripts/bootstrap.sh          # 系统库 + pnpm + cargo check + 生成
pnpm install --frozen-lockfile
zk compose --edition personal --profile default --channel official   # 生成两端 glue（--channel 必填；--profile 默认 default）
```

## 6. Build and test commands

### 【当前可用】构建/测试

```bash
# 构建期换包 POC：三种 profile 换同一角色
cd experiments/composition-poc
python3 resolve.py default     && cargo run -q -p zk   # plain fs
python3 resolve.py hardened    && cargo run -q -p zk   # sandboxed fs
python3 resolve.py vendor-acme && cargo run -q -p zk   # workspace 外实现

# 文档 / 变更规格门禁
python3 scripts/docs-check.py
python3 scripts/spec-check.py
python3 scripts/review-check.py
bash scripts/new-change.sh <kebab-slug>   # 新建 change 五件套

# WASM 应用 POC：host + Go/Rust guest + 帧 ABI + SQLite + fuel
cd experiments/wasm-poc
./build.sh                      # 首次约 6–7 分钟（wasmtime），构建后自动跑 golden
python3 golden/run.py           # 可选：已 build 后单独重跑，期望 {"ok": true, ...}
```

### 【scaffold 后】构建/测试

```bash
# TypeScript
pnpm -r typecheck                # tsc -b（最严格）
pnpm -r lint                     # eslint --max-warnings 0
pnpm -r test:unit                # vitest（含 typecheck）
pnpm test:e2e                    # playwright（serve 载体）
pnpm -r build

# Rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo nextest run --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo deny check
cargo machete

# 组合与生成物
zk validate
zk compose --edition personal --profile default --channel official --check
zk compose --edition enterprise --profile default --channel official --check
zk api --edition personal --profile default --channel official --check
zk capability --check
zk docs --check
python3 scripts/review-check.py
zk db lock --check
cargo test --workspace export_bindings && git diff --exit-code
cargo test --workspace export_schema && git diff --exit-code
bash scripts/schema-check.sh
```

**完成判据**：对应命令在 CI 绿 + 证据文件存在 + 可复现；模型自述不算。

## 7. Code style

### Rust

- workspace lints 全开（`docs/18` §4）：`unsafe_code=forbid`、`clippy all/pedantic/nursery/cargo`,
  `unwrap_used/expect_used/panic/indexing_slicing/arithmetic_side_effects/as_conversions=deny`。
- 测试模块可局部 `#[allow(clippy::unwrap_used, clippy::expect_used)]`，必须行内注明。
- 金额/计数用 `checked_*`/`saturating_*`；索引用 `.get()`；转换用 `TryFrom`；禁止裸 `as`。
- 库用 `thiserror`；边界用 `anyhow`；**每个 crate `#![forbid(unsafe_code)]`**（零例外，禁止 `#[allow(unsafe_code)]`）。
- 需要底层能力（WASM 运行时、系统 API）时，使用经过审计的依赖（wasmtime/tauri 等），**不自己写 unsafe**；
  依赖内的 unsafe 由锁定版本 + `cargo-geiger`/`cargo-deny` 审计管理（见 `docs/18` §4）。
- 包 `name()` 必须命名空间化（`<role>/<package-id>`），避免 cordis 同名 runtime 合并。
- 可复用逻辑只进 `crates/zk-kit-*` 或 `@zeokeel/ui-kit`；包内 helper 私有；禁止新建 `utils/helpers/common`。
- 可复用公共项必须有 `@capability <id>` 注释；`zk capability --check` 必须绿。
- 插件/服务必须把资源包成 effect（任务、监听器、fd），否则内核回收不到。

### TypeScript / React

- `docs/18` §1–§3：`strict` + 全部严格附加项；禁 `any`、禁 `as`（`as const` 除外）；
  `eslint --max-warnings 0`；边界用 Ajv 按 Rust 生成的 JSON Schema 校验。
- 组件只读 reducer/view model；流式事件进纯 reducer；未知事件忽略并记日志。
- 不自己写 Markdown 解析/虚拟滚动/焦点管理；用 §3.1 指定库。
- 每个 UI 组件必须覆盖六态：空/加载/错误/离线/流式中/已取消。
- 共享组件/格式化进 `@zeokeel/ui-kit`；业务组件不得被其它包 import。

### 包与命名

- 包 id：`zeokeel/<name>`；Rust crate：`zk-<name>`；服务端 crate：`zk-srv-<name>`；管理台页面 crate：`zk-admin-<page>`；UI npm：`@zeokeel/ui-*`/`@zeokeel/admin-*`。
- apps 入口 crate：`zk-desktop`/`zk-server`；引导 crate `zk-bootstrap`、生成 glue `zk-composed`（`crates/`，ADR-009）。
- 目录：`packages/<group>/<name>/`，UI facet 在包内 `web/`。
- 错误码封闭集合（`shared/contracts/errors.json` 生成）；文案走 i18n，不硬编码中文/英文。
- 数据兼容：新增字段 `#[serde(default)]`；前端类型可选；禁止破坏已有字段。

## 8. Testing instructions

- 证据政策见 `docs/11-testing-and-evidence.md`：只有可执行、可复现、外部化的证据算完成。
- 分层：T0 组合快照（秒）→ T1 单包 → T2 组合构建 → T3 集成（真 PG/真 IPC）→ T4 矩阵（edition/OEM）→ T5 交付物。
- 客户端：reducer golden traces（含中文/emoji 跨 UTF-8 分片）、取消/重连/IME/滚动/Markdown 未闭合/主题截图/axe。
- 服务端：迁移不可变（改历史必红）、全量端点权限矩阵、钱包并发与峰谷结算、网关流式、WASM 限额。
- 新增/修改行为必须加测试；修 bug 先写复现。
- 不许用"手动点过"代替测试；手测要落成脚本。

## 9. Security considerations

- 禁止提交任何密钥/token/客户名/OEM 私有数据；OEM 内容只在私有索引仓/客户子仓。
- 服务端 `master.key` 只存部署目录；上游 provider key、MCP secret 加密存储并掩码返回。
- 企业版客户端不得持有上游 key；AI 只经网关；禁用用户令牌立即失效。
- 更新：个人版只从 GitHub Releases；企业版客户端只从登录服务端（1:1）更新；服务端更新必须人工备份 + `docker load` + compose；禁止自动升级。
- WASM 应用：无网络/文件/env，仅宿主 `db.*/log/asset.read`；新增 host function = ABI 变更，需版本化 + 审计。
- 审计写入失败时业务 fail-closed；审计/用量保留期不得低于 180 天/12 个月。
- Windows 安装包**不签名**（已定）；macOS 公证 CI 先就绪、证书后配（见 §12 TODO）。
- 弱管控：不要在文档/销售中宣称"可限制员工操作"；安全责任声明见 `docs/00` §5。

## 10. Documentation rules

- 唯一入口：[`docs/README.md`](docs/README.md)（SSOT/术语/任务路径）；根 `README.md` 只做简介与 10 秒入口。
- 权威顺序：**用户当前指令 > 本文件 > `docs/00`/`docs/15` > `docs/19–24`（域内补充规范）> `docs/PLAN` > 其他 docs > research（只作事实参考）**；域内 SSOT 以 `docs/README.md` §2 为准。
- 冲突时停下来问用户，不要自行选一个。
- 文档里的数字/索引/契约表必须由生成器产出（scaffold 后 `docs/generated/`），禁止手写计数。
- 改行为必须同步改文档；新增 ADR 放 `docs/13`，决策变更先改 ADR 再改实现。
- 生成物头部 `@generated`，禁止手改；`--check` 失败时改生成器或输入，不改产物。
- **ADR 状态机**：决策被取代时，旧 ADR 必须标 `superseded-by-ADR-0xx` 并回写所有引用/重复表述；禁止两篇 `accepted` ADR 互相矛盾。
- **唯一已定清单**：`docs/15`（D1–D67；实现细节待定索引见 §C）+ `docs/13`（ADR-001…058，其中 ADR-013 为显式 `open` 的非本期扩展点）+ `docs/19–28`（域内规范）是已定事项真源；PLAN/其它文档只引用编号，不手抄结论。
- **变更规格**：`specs/changes/<id>/` 五件套；`spec-check` 门禁；耐久结论回写 `docs/`，spec 归档。
- **Skills**：`.agents/skills/` 只写步骤/命令/判据，不复制规范；冲突时以本文件与 `docs/README.md` 为准。

## 11. 明确禁止（Do NOT）

| 禁止 | 原因 |
| --- | --- |
| 加回强管控（工具白名单、网络限制、企业审批、部门/用户组） | 已决策弱管控（ADR-030/032） |
| 支持 Chat Completions（企业网关） | 只支持 Responses（ADR-031） |
| 做 Windows 代码签名 | 已决策不做（ADR-033） |
| 运行时插件加载 / dylib / 热插拔 / 运行期切换 edition/品牌 | 构建期组合（ADR-001/020/024） |
| 服务端执行 agent、执行技能、代理 MCP 流量、多租户、部门预算 | 非目标（`docs/07` §0.1） |
| 用 `any`/`as`（除 `as const`）、`unwrap/expect/panic`、索引切片、裸算术 | 严格检查（`docs/18`） |
| 写 `unsafe` / `#[allow(unsafe_code)]` / 为性能放宽 forbid | 零 unsafe 政策（C9b） |
| 关闭/放宽全局 lint、文件级 blanket allow | 同上 |
| 手改 `@generated` 文件/生成索引/契约 JSON | 生成器唯一所有权 |
| 跨包直接依赖具体实现类型（而非契约） | 可替换性（ADR-002/004） |
| 提交密钥、客户信息、OEM 私有素材到公开仓 | 安全/合约 |
| 把参考仓库（channels/Harness/Next）当依赖/构建输入/索引仓/数据源 | C15 |
| 重复实现已有能力：本地 `utils/helpers`、复制近似代码、同一职责两套实现 | C17/`25` |
| 业务包 import 其它业务实现；kit 反向依赖业务包 | C4/C17 |
| 新增/修改共享能力但未登记 `@capability`/未更新能力索引 | C17/`25` |
| 把"未验证"说成"已验证"；用模型自述当证据 | 证据政策 |

## 12. 当前 TODO / 已知未决

- macOS 公证：**本地先测**，证书后配，CI 先就绪（ADR-037）。
- S0：P2（WASM）原型完成但**证据未归档**（M0 前补跑 `build.sh` + `golden/run.py` 并存 `target/evidence/`）；P1 双 app 组合、P3 认证+JWT、P4 OEM submodule CI、P5 迁移不可变、P6 管理台包化待做（S0 阻塞 M0，见 `docs/12`）。
- 实现细节待定：统一索引 `docs/15` §C（T1–T21），不阻塞 spec 编写与机制 spike；实现前在对应 milestone spec 冻结。
- 复用/去重门禁：能力索引、路径/import 检查、克隆检测与 baseline，M0 落地（`25`、T20/T21）。
- 已拍板（v3.2）：余额=企业配额、不做开票/收款、OEM 代码自研、Powered by OEM 默认隐藏、不做 DPA、暂不定义 RPO/RTO、安全内部评审（D60–D65）。
- 已定清单：`docs/15-open-questions.md`（D1–D67）；决策理由：`docs/13-risks-and-adr.md`；`docs/17-v3.1-decisions.md` 是历史存档，不作依据。

## 13. Agent workflow（怎么干活）

1. **先读**：本文件 → `docs/README.md`（SSOT/任务路径）→ `specs/README.md`（变更规则）→ 任务对应权威文档 → `docs/18`/`docs/25`（写代码时）。
2. **先搜复用**：`zk find <能力>` / 读 `docs/generated/CAPABILITY-INDEX.md` / `rg` 现有符号；确定复用、扩展或新建，并留证据（`25` §5）。
3. **先写文档 + 评审**：发现新方案/设计/模式时，先落 `docs/` 或 ADR；由非实现者只读评审 → 修复 → 复评通过（`11` §9、`27` §5）。
4. **先查 spec**：`specs/changes/` 是否已有同目标变更；没有则 `bash scripts/new-change.sh <slug>`，写 `proposal.md` 并让用户确认后改 `approved`。
5. **先问**：需求/契约/数据格式/安全边界不明确时问用户；不要替用户做产品决策。
6. **小步**：按 `tasks.md` 一次一个可提交步骤；改契约先跑 conformance 与 public-api diff。
7. **验证**：按 §6 跑对应命令；失败就修到绿，不许跳过。
8. **收尾**：填 `acceptance.md`；更新文档/ADR/README；检查 `docs-check`/`spec-check`/`zk capability --check` 与生成物 `--check`；归档 spec。
9. **提交**：Conventional Commits（`feat(scope): ...`）；一个 PR 一件事；PR 描述链接 `specs/changes/<id>`，并回答复用四问。

## 14. PR / commit 约定

- 标题：`<type>(<scope>): <summary>`，type ∈ feat/fix/refactor/docs/test/chore/perf/build/ci。
- 描述必须写：改了什么、为什么、验证命令与结果、影响的包/契约/数据。
- 契约/数据/安全相关变更必须附 ADR 或文档链接。
- 禁止提交 `target/`、`node_modules/`、生成物之外的构建产物、密钥。
