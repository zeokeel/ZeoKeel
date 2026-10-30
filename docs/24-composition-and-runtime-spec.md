# 24 · 组合、Manifest 与运行时契约（补充规范）

> 状态：Plan v3.2 补充规范。补齐架构审计中的系统级 P0：
> **组合优先级、manifest schema、生成物所有权、路由真源时序、迁移命名、关闭状态机、客户端会话存储、metadata 策略**。
> 本文冻结"实现契约"；与 `02/03/05/06/10` 冲突时，以本文为准并回写旧文。

## 1. 组合层与优先级（冻结）

### 1.1 层顺序（低 → 高）

```text
base_edition 链（personal）
  → edition（enterprise 继承 personal + exclude）
    → profile（环境覆盖：default/mock/minimal/...）
      → channel（official/beta/oem-*；品牌/默认值，可追加/覆盖包）
```

- 同一层内重复声明同一 singleton 角色 = **错误**（不是后者覆盖）。
- 跨层：后层覆盖前层；覆盖必须显式（不允许"同名猜测"）。
- `exclude` 只作用于**继承的 base**，值必须是**包 id**（`<vendor>/<name>`），不是角色名；
  未命中 base 闭包 = 错误；`personal-only` 清单单独文件（`editions/personal-only.toml`），CI 断言企业版闭包不含其中任何包。
- multi 角色：**不在 `[packages]` 内单点覆盖**；增删用 `[packages.add]`/`[packages.remove]`，不允许隐式并集。
- profile 只允许改环境/设置/覆盖 singleton；删除包必须用 `exclude`（edition）或 `remove`（profile/channel），不得靠"不写"。

### 1.2 基线元组（入库）

- 唯一入库生成物基线：`edition=personal × profile=default × channel=official`。
- 其它组合（enterprise、oem-*）由 CI 用 `--check` 生成到临时目录比对，不二次入库。
- `zk compose/api` 的 `--channel` 必填（无默认），`--profile` 默认 `default`；`zk db` 只处理迁移，不需要 channel；生成物头部写 `@edition/@profile/@channel`。

### 1.3 解析输出（resolve report）

每次 compose 生成 `resolve-report.json`：层输入哈希、每角色最终包、来源层、被覆盖/排除项、闭包差异。
T0 快照比对；任何"静默变化"= 红。

### 1.4 失败行为

- 未知角色、未知包、契约不匹配、exclude 未命中、同层重复、channel 追加未声明包 → **非零退出**，无部分产物。
- `[packages.remove]` 未命中、`[packages.add]` 的包已在该角色提供者中、同一角色同时出现在 add 与 remove → **错误**。
- multi 最终顺序 = 基线数组顺序 + add 追加（remove 后保持相对顺序）；无显式顺序需求时按包 id 排序。
- 包声明的每个角色必须被该层显式映射才激活；未映射的角色不自动提供（不隐式激活多角色包）。
- 不允许"警告后继续"。

## 1.5 角色注册表（root `zk.toml [roles]`）

M0 起点角色表（`[roles]` 的键；提供者声明的 contract/cardinality 必须与本表一致；完整名单由 M0 spec 冻结，T2）：

| 角色 | 契约 | 基数 | editions | 说明 |
| --- | --- | --- | --- | --- |
| `agent.loop` | `zk-contract-agent` | singleton | personal, enterprise | 本地 agent 循环唯一 |
| `agent.prompt.section` | `zk-contract-agent` | multi | personal, enterprise | 提示词片段 |
| `auth.client` | `zk-contract-auth-client` | singleton | enterprise | 客户端登录/刷新（个人版无） |
| `host.fs-access` / `host.process` / `host.net` / `host.secrets` | `zk-contract-host` | singleton | personal, enterprise | 宿主能力 |
| `host.tasks` / `host.clock` / `host.config` | `zk-contract-kernel` | singleton | personal, enterprise | 内核 Clock/Tasks/Config 的宿主服务（`05` §2） |
| `platform.shell` / `platform.http` | `zk-contract-platform` | singleton | personal, enterprise | 桌面壳 / serve 载体；与前端 `ui.transport` 成对 |
| `platform.ipc` | `zk-contract-ipc` | singleton | personal, enterprise | IPC 通道（`Command`/`Event`/`StreamChannel`） |
| `ui.shell` | `zk-contract-ui` | singleton | personal, enterprise | 外壳唯一 |
| `ui.shell-kit` | `zk-contract-ui` | singleton | personal, enterprise | 壳原语 + `SLOT_DEFS`/props 常量（`16` §3） |
| `ui.transport` | `zk-contract-ui` | singleton | personal, enterprise | 前端 transport（tauri/http/mock） |
| `ui.slot.<slotSuffix>` | `zk-contract-ui` | multi | personal, enterprise | 槽位贡献；slotId = `ui.<slotSuffix>`，清单见 `16` §3 |
| `store.session` / `store.settings` | `zk-contract-session` / `zk-contract-config` | singleton | personal, enterprise | 持久化实现 |
| `tool.<name>` / `tool.group.<name>` | `zk-contract-tool` | singleton / multi | personal, enterprise | 工具；工具名唯一 |
| `llm.provider.default` | `zk-contract-llm` | singleton | personal, enterprise | 个人版=本地 provider，企业版=网关 provider（构建期覆盖） |
| `llm.provider.<vendor>` | `zk-contract-llm` | multi | personal | 多供应商并存（M3 起） |
| `theme.tokens` / `brand.mark` / `i18n.*` | `zk-contract-ui` | singleton | personal, enterprise | 换肤/白标/词条 |
| `client.update` | `zk-contract-update` | singleton | personal, enterprise | 更新源实现（GitHub Releases / 登录服务端） |
| `srv.auth.source` / `srv.auth.session` | `zk-contract-auth` | singleton | enterprise | 认证源与登录会话 |
| `srv.users` | `zk-contract-user` | singleton | enterprise | 用户/角色仓储（无部门） |
| `srv.wallet` / `srv.pricing` / `srv.job.wallet-grant` | `zk-contract-wallet` | singleton | enterprise | 钱包/价格/发放作业 |
| `srv.gateway` / `srv.gateway.upstream` / `srv.gateway.meter` | `zk-contract-gateway` | singleton | enterprise | 网关/上游适配/计量 |
| `srv.content` / `srv.skills` / `srv.apps` | `zk-contract-content` / `zk-contract-skill` / `zk-contract-app` | singleton | enterprise | 内容发布内核/技能/应用 |
| `srv.connectors` | `zk-contract-connector` | singleton | enterprise | MCP 下发 |
| `srv.audit` / `srv.settings` / `srv.jobs` / `srv.http` | `zk-contract-server` | singleton | enterprise | 审计/配置/调度/HTTP |
| `admin.shell` | `zk-contract-admin-ui` | singleton | enterprise | 管理台壳（导航/布局/主题） |
| `admin.page.*` / `admin.slot.*` | `zk-contract-admin-ui` | singleton / multi | enterprise | 管理台页面与槽位 |

```toml
[roles."llm.provider.default"]
contract = "zk-contract-llm"
cardinality = "singleton"
editions = ["personal", "enterprise"]
protected = false          # true = OEM 不得替换

[roles."srv.auth.source"]
contract = "zk-contract-auth"
cardinality = "singleton"
editions = ["enterprise"]
protected = true           # 安全边界，OEM 不得替换
```

规则：未知角色=error；提供者声明的 cardinality/contract 必须与注册表一致；`protected=true` 的角色 OEM 覆盖=error；
角色命名统一 `auth.client`（客户端登录）/`srv.auth.source`（服务端认证源），禁止 `auth.provider` 别名。
上表是 M0 起点：新增/改名角色必须先改本表再进实现（C12）；`zk validate` 对未登记角色报错。
multi 角色允许 0 个提供者；singleton 必须恰好 1 个（`optional` 名单由 T2 冻结）。以下包的服务角色由 T2 补登记，
当前按包内服务处理：`runtime`/`timer`/`task-service`/`supervisor`/`telemetry`/`approval`/`audit`、
`client-bootstrap`/`client-skill`/`client-connector`/`client-wasm-host`。

## 2. `zk.toml` Package Manifest v1（冻结字段）

> 以下代码块合并展示字段：实际包按 `kind`/facet 取舍（客户端包用 commands/ui.contributions，服务端包用 routes/jobs），不是每个包都有全部表。

```toml
schema = 1
id = "zeokeel/tool-fs"              # 唯一；<vendor>/<name>
kind = "capability"                 # contract/runtime/platform/capability/ui/product/server/admin-ui
version = "0.1.0"
license = "MIT"
summary = "工作区文件读写工具"
stability = "beta"              # stable | beta | experimental


[role]
name = "tool.fs"                    # 单角色用 [role]，多角色用 [[roles]]；二者只能出现一种
contract = "zk-contract-tool"       # 必填
cardinality = "singleton"           # singleton | multi
editions = ["personal", "enterprise"]
required = true                     # 启动失败策略：缺包则诊断页退出
swap_required = false               # 是否允许 OEM 深度替换

[config]
schema = "schema/config.json"        # schemars 生成

[permissions]
declares = ["fs:read", "fs:write"]   # 粗粒度审批声明（权限点用 <域>:<动作>），不是安全边界

[compat]
platforms = ["linux-x64", "macos-arm64", "windows-x64"]
kernel = "zeokeel-cordis@0.6"

[conformance]
suite = "zk.contract.tool.conformance@1"

[requires]                          # 只写角色，不写包名
"platform.shell" = "singleton"
"tool.fs" = "singleton"

[facets]
rust = { crate = "zk-tool-fs" }
ui = { package = "@zeokeel/ui-tool-fs" }

[[commands]]                        # 客户端 IPC 命令（静态声明）
name = "tool.fs.read"
args_schema = "contracts/tool-fs/read.schema.json"
permission = "fs:read"

[[routes]]                          # 服务端路由静态真源
method = "GET"
path = "/api/client/v1/tools"
api_version = "v1"
auth = "bearer"
permission = "tools:read"
body_limit = "1MiB"
rate_limit = "default"        # 限流类，必须声明
step_up = "none"              # none | confirm
idempotency = "safe"
audit = false

[[ui.contributions]]                 # UI facet 的静态贡献（slotId 见 16 §3）
slot = "ui.settings.section"
component = "tool-fs.settings"
order = 20
props_schema = "schema/tool-fs.settings.props.json"

[[jobs]]                             # 后台作业（仅服务端包；示例仅示字段）
name = "wallet.usage.sweep"
schedule = "*/5 * * * *"
idempotency = "job:wallet.usage.sweep:{run_at}"
timeout = "60s"
on_error = "retry"

[migrations]
dir = "migrations"                  # 文件由 zk db lock 全局编号
```

规则：

- 未知键/未知枚举 = error；`required`（启动失败策略）只在 `[role]` 上声明。
- `[role]` 与 `[[roles]]` 只能二选一；每个 role 元素字段相同。
- 命令/路由**不得在源码里手写第二份**；运行期 `Register()` 只做一致性校验（见 §4）。
- 管理台页面也声明 `[[ui.contributions]]`（slot/component/order/props_schema）；页面角色与权限点见 `08` §3。
- `[jobs]` 统一用 `[[jobs]]`（name/schedule/idempotency/timeout/on_error），禁止无 schema 的键值占位。
- edition 归属只写在 `[role].editions`；ADR-020 早期示意的包级 `[editions]` 表已废弃，以本节为准。
- 版本/源/哈希由 `zk.lock` 解析，不在 manifest 手写路径/版本范围。

### 2.1 根 `zk.toml`（组合根清单 v1）

根清单是**唯一组合输入**；字段骨架如下，最终字段/默认值由 M0 spec 冻结（T2，见 `15` §C）：

```toml
schema = 1

[scan]                        # 包索引扫描根；workspace 内包 + vendor + OEM 子仓
packages = ["packages/*/*", "vendor/*", "oem/*/packages"]   # OEM 子仓布局见 09 §2，最终由 T2 冻结

[paths]                       # 生成物根（具体文件见 §3）
crates = "crates"
apps = "apps"
shared = "shared"
docs_generated = "docs/generated"

[oem]
dir = "oem"                   # submodule 挂载；索引仓格式由 M0 spec 冻结

[personal_only]
file = "editions/personal-only.toml"   # 企业版闭包不得包含其中任何包 id

[roles."llm.provider.default"]
contract = "zk-contract-llm"
cardinality = "singleton"
editions = ["personal", "enterprise"]
protected = false

[contracts]                   # 契约 id ↔ crate 唯一映射（02 §5）
"zk.contract.package" = "zk-contract-package"
"zk.contract.llm" = "zk-contract-llm"
```

规则：`[roles]` 全量表见 §1.5；`[contracts]` 是契约 id ↔ crate 名的唯一映射，双向校验（`02` §5）；
`editions/personal-only.toml` 只列包 id；未知键/未登记角色/未登记契约 = error。

## 3. 生成物所有权（一路径一 writer）

| 产物（路径） | 唯一 writer | `--check` 行为 |
| --- | --- | --- |
| `crates/zk-composed/{Cargo.toml,src/lib.rs}`（Rust glue） | `zk compose` | 生成到临时目录逐字节比较，不写 |
| `apps/server/src/generated/`、`apps/admin/web/src/generated/` | `zk compose` | 同上 |
| `apps/desktop/web/src/generated/`（UI registry、路由/命令表） | `zk compose` | 同上 |
| `shared/contracts/{api-catalog,http-routes,openapi,permissions}.json` | `zk api`（读 `[[routes]]`） | 同上 |
| `shared/contracts/{ipc-commands,ui-contributions,ui-slots,roles,packages,i18n-catalog}.json` + `shared/ts/i18n.gen.ts` | `zk compose` | 同上 |
| `apps/{desktop,server}/Cargo.toml` | `zk compose`（块内） | 块外不覆盖 |
| `zk.lock` | `zk lock` | `zk verify-lock` 只校验 |
| `metadata.snapshot.json` | `zk metadata snapshot`（离线构建用） | 只校验 |
| `edition.gen.{rs,ts}` | `zk compose` | 同上 |
| `shared/contracts/errors.json` | `zk api`（从契约错误码枚举生成，`04` §7/§10） | 临时目录比较 |
| `shared/contracts/events.json` | `zk compose`（事件 topic 注册表，`04` §8） | 同上 |
| `shared/ts/*.ts` | `cargo test --workspace export_bindings`（ts-rs） | `git diff --exit-code` |
| `shared/schema/*.json`、包内 `schema/*.json` | `cargo test --workspace export_schema`（schemars） | 同上 |
| `docs/generated/*.md` | `zk docs`（数据来自 `zk plan`/`zk api`/`zk db lock`） | 临时目录比较，不写 |
| `migrations.lock` | `zk db lock`（`--check` 只校验） | 不写 |
| 根 `Cargo.toml` 的 `[patch]`/`[workspace.lints]` | `zk compose` 在 `@generated:begin/end` 块内 | 块外内容不覆盖 |
| `deploy/docker/Dockerfile` 生成段 | `zk compose` 生成块 | 手写段禁止改生成块 |

- 禁止 `zk` 与 ts-rs/schemars 同时写同一路径；混合文件用生成块。
> 本表为 M0 起点；最终 writer/路径由 M0 spec 冻结（T3），冻结后不得再增第二 writer。

- 生成物头部 `@generated` + 输入哈希 + `@edition/@profile/@channel`；入库基线见 §1.2（`personal×default×official`），
  其它组合只在 CI 临时目录生成；`--check` 永不写源文件。

## 4. 路由/命令真源时序

- **静态真源**：`[[routes]]` / `[[commands]]` 在 manifest 中声明；`zk api` 在**不编译**的情况下生成 catalog/permissions（T0 可跑）。
- **运行期校验**：服务端 `Register()` 读取生成表，与包实际注册的路由逐项比对；缺 `api_version/rate_limit/step_up` 或多/少/方法冲突/路径冲突 → **拒启**。
- 中间件顺序固定：request_id → auth → rate limit → body limit → permission → audit → handler；不允许包自定义全局顺序。
- 客户端 IPC 同样：`commands.gen.ts` 与主进程 handler 表比对，缺一拒启/构建失败。

## 5. 迁移命名与执行

- 全局单调版本：`zk db lock` 分配 `0001_...`、`0002_...`；每个迁移只属于一个包。
- `zk db lock --check` 只校验已有哈希/顺序、不写文件（CI 漂移门禁用）；`zk db lock` 只追加，不改历史。
- `schema_migrations(package, version, checksum, applied_at)` 复合键；全局序号保证顺序。
- 启动检测：重复版本/缺号/checksum 变化/顺序漂移 → 拒启（只允许新增）。
- 升级采用 expand/contract；`zk db lock` 不是"洗白"工具：校验历史文件哈希，只允许追加。
- 回滚：N-1 镜像必须能读新 schema（expand 阶段）；contract 清理只能在 N+1 之后。

## 6. 服务端关闭状态机（SIGTERM）

```text
SIGTERM
 → 停止 accept 新连接
 → 等待在途请求（deadline 30s）；SSE 发 error/end 并触发结算
 → 结算所有 pending usage/hold
 → flush 审计（fail-closed）
 → 停止 cron/jobs（等待当前 job 完成或标记中断）
 → 关闭 PG 池
 → 退出 0
超时 → 强制退出 1，残留 pending 由启动补偿处理
```

- 启动取 PG advisory lock；拿不到=拒启（防多实例重复发放/退役）。
- pending 结算表 + 启动扫描：超时未结算 → 按最后 usage 结算或 void，差异进对账。

## 7. 客户端会话存储（冻结）

- **SQLite WAL 唯一真源**：`conversations/messages/turns/tool_calls/approvals/checkpoints`；schema v1 + 迁移。
- 一个 turn 原子提交：user message + assistant partial/final + tool calls + usage 同事务。
- 中断：`turn.status=interrupted`，保留已生成部分；重开可继续/重试；checkpoint 周期落盘。
- 单实例锁（文件锁）；多窗口只读订阅；崩溃恢复扫描未完成 turn。
- JSONL 仅导入/导出，不作为运行时存储。

## 8. `cargo metadata` 策略

- 凡声称"契约单实例/包身份"的命令（validate/compose/api/sbom）必须有 metadata；失败=非零，禁止降级。
- 离线构建使用 `metadata.snapshot.json`（入库）并校验工作区哈希；`--no-verify` 仅本地，CI 禁用。
- `cargo metadata` 只做交叉校验；包索引以 manifest 为准；两者不一致 = 红。

## 9. 验收

**M0（组合与生成物）**

- [ ] 同一输入重复 compose 生成物逐字节一致；`--check` 不写文件。
- [ ] 企业版闭包不含 personal-only 包；exclude 未命中报错。
- [ ] 路由/命令静态表与运行期注册不一致时拒启。
- [ ] 迁移重复/改历史/顺序漂移均红；`zk db lock --check` 只读、只能追加。

**M1/M2（运行时）**

- [ ] SIGTERM 演练：流式请求结算、审计 flush、pending 补偿。
- [ ] 客户端崩溃恢复：turn 原子性、interrupted 恢复、单实例锁。
