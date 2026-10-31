# 06 · 服务端运行时：进程、包、数据库、迁移、调度

> 服务端和客户端用**同一套包模型与组合器**（ADR-021），但运行时形态不同：客户端是桌面进程，
> 服务端是单进程 HTTP 服务。本文件只讲"服务端这个宿主"。
> 各业务域见 `07-server-domains.md`；API 契约见 `04-contracts-and-api.md`。

> 运维/Day-0/备份/升级见 `20`；安全与密钥见 `21`；关闭 FSM 与迁移命名见 `24` §5–§6。

## 1. 进程模型

```text
zeokeel-server（单二进制；crate 名 `zk-server`）
├─ bootstrap（唯一非包代码）
│   读 .env/系统配置 → 初始化 tracing → 连接池 → 跑迁移 → 创建 cordis root
│   → 注册服务端包（拓扑序）→ 构建 axum Router → 启动后台调度器 → listen
├─ axum（唯一 HTTP 入口）
│   /api/client/v1/*     员工客户端 API
│   /api/admin/*         管理台 API（Cookie session + CSRF）
│   /v1/*                AI 网关（Bearer JWT）
│   /portal/*, /api/updates/* 门户与客户端安装包下载
│   /admin/*             管理台静态资源（内嵌）
│   /healthz, /readyz, /metrics
├─ cordis root（zeokeel-cordis）
│   包 = Plugin：注册服务（DB 仓储、配置、事件、任务、缓存）+ 路由贡献 + 迁移 + 后台作业
├─ 后台调度器（进程内）
│   月发放、应用退役、令牌/用量/审计清理、客户端版本检查
└─ 进程内基础设施：有界任务队列、TTL 缓存、事件总线（无 Redis/MQ/S3）
```

- **实现状态（task-13）**：`apps/server/src/main.rs` 在连接池之前调用
  `zk_bootstrap::boot("zk-server")` 创建 kernel root，并把 `zk_kit_server::AppState` 注册为 kernel
  service、jobs loop 由 `Bootstrap::spawn_task` 监督；SIGTERM drain + 结算 + 审计校验后调用
  `Bootstrap::shutdown()` 逆序回收 effects（`docs/24` §6）。

设计约束（参考实现的取舍，已确认沿用）：

- **无外部中间件**：不引入 Redis/MQ/S3。缓存用进程内 TTL；队列用有界 in-process 队列；
  技能归档/应用包直接进 PG `BYTEA`（大对象另议）。
- **单实例**：不做集群/HA。升级 = 停 → 备份 → 迁移 → 起。需要时用外层负载均衡做多副本只读（不在本期）。
- **单租户**：一套部署一个企业，无 `tenant_id`。
- **密钥只在服务端**：上游 provider key、MCP secret、JWT 签名密钥、`master.key` 都不下发客户端。

## 2. 服务端包契约

服务端包与客户端包共享 `zk-contract-package`（`descriptor()` + `install()`），额外实现
`zk-contract-server`：

```rust
pub trait ServerPackage {
    fn descriptor(&self) -> PackageDescriptor;              // 包元数据
    fn migrations(&self) -> &'static [Migration];           // 不可变迁移（含内容哈希）
    fn jobs(&self) -> Vec<JobContribution>;                 // 后台作业（幂等）
    fn config_schema(&self) -> Option<&'static str>;        // JSON Schema（系统配置项）
}
```

规则：

1. **路由静态真源（计划/未实现）**：`[[routes]]` 在 manifest 声明，`zk api` 生成 catalog/permissions；运行期 `Register()` 只做一致性校验（多/少/冲突→拒启）。规范见 `24` §4、ADR-047；当前以各包 `routes()` 为真源，`zk api` 未实现。
2. **权限点同处申报**：权限常量是枚举（超管/审计员/员工），路由注册时一并声明，CI 生成"全量端点权限矩阵"。
3. **包之间只经契约交互**：服务（`ctx.get`）或事件；不得直接依赖别的服务端包的仓储类型。
4. **迁移归属包**：每个包自己的表由自己迁移；跨包只经契约。表名带包前缀（`auth_*`、`wallet_*`）。
5. **作业必须幂等**：调度器可能重试；每次执行写 `job_runs` 记录（job、key、状态、耗时）。钱包相关作业至少：`wallet_hold_expire`、`usage_pending_sweep`、`wallet_reconcile`、`usage_month_snapshot`（规则见 `22`）。
6. 包 `descriptor` 里声明启动失败策略：`required=true` 失败则拒绝启动（诊断页），否则 degraded 并在管理台显示。

## 3. 数据库与迁移（ADR-022）

- **PostgreSQL only**（应用内 SQLite 除外，见 WASM 域）。
- 迁移文件随包发布，**全局版本号由 `zk db lock` 分配**（不再每包 `0001`）；目录规则见 `24` §5。
- 迁移不可变 + 全局版本/顺序/复合键规则见 [`24` §5](24-composition-and-runtime-spec.md#5-迁移命名与执行)；本节只保留包内目录与启动校验入口。
- 数据库结构变更只由生成器重新生成 `docs/generated/schema.md`：唯一 writer 是 `zk docs`（数据来自 `zk db lock`，见 `24` §3）；`zk db lock` 只写/校验 `migrations.lock`，`zk db lock --check` 只校验不写。
- 开发期允许 `zk db reset`（未实现，backlog）；生产只允许迁移。
- 分区：`usage` 按月、`audit_log` 按月；账本永久保留。
- 连接池：单进程 `sqlx::PgPool`；禁止"持事务再要连接"的嵌套获取（参考实现死锁事故）。

## 4. 认证边界（三种入口，互不复用中间件）

| 入口 | 凭据 | 用途 | 说明 |
| --- | --- | --- | --- |
| `/api/client/v1/*` | `Authorization: Bearer <JWT>` | 员工客户端 | 短效 access；refresh 走单独端点 |
| `/v1/*` | 同上 | AI 网关 | 先鉴权再准入（余额/限流/模型定价） |
| `/api/admin/*` + `/admin/*` | Cookie session + CSRF | 管理台 | 超管（读写）/审计员（只读）；TOTP MFA 可选（D53） |

- JWT 签名密钥由服务端生成/配置（轮换 `kid` 为部署规划项；**不由 `master.key` 派生**，与上游 key 加密分离）。
- 员工禁用/降权 → `token_epoch++`，每请求校验（写操作强制查库），**立即失效**，不等 access 过期（见 07 §1）。
- 管理台与员工 API 的路由物理分离，避免"员工令牌调管理接口"。

## 5. 配置

- `.env` 只放 6 个必填键（与 `09` §5 一致）：`DOMAIN`、`TLS_MODE`、`ADMIN_PASSWORD`、`PG_PASSWORD`、`SERVER_IMAGE`、`MASTER_KEY`；反代场景可选覆盖 `ZEOKEEL_PUBLIC_BASE_URL`（第 7 个，非必填）。
- 其余运行期配置进 `system_settings` 表，管理台可改；每项标注 `hot`（立即生效）或 `restart`（提示重启）。
- 配置读取经 `zk-contract-config` 的 `ConfigService`；包不得直接读环境变量（除 bootstrap 注入的少数）。
- 上游 provider key **已实现** AES-256-GCM 版本化加密存储（`crates/zk-kit-server::master_key`）：
  密文格式 `vN:<base64(nonce||ct)>`；密钥来源 `ZEOKEEL_MASTER_KEY`（32 字节 hex/base64，版本 `ZEOKEEL_MASTER_KEY_VERSION`，默认 1）
  或 `<ZEOKEEL_MASTER_KEY_DIR>/master.key[.vN]`（0600，拒绝组/全局可读；`master.key` = v1）。新写用最高版本、解密按前缀选版本；
  旧明文字段可读但告警，下次写入即加密；无密钥时新写 fail-closed（`MASTER_KEY_UNAVAILABLE`）。接口只返回掩码。
  MCP 静态 secret 加密/受控交付延后 M3.1（ADR-060）。轮换流程见 `21` §3 + `scripts/master-key-rotate.sh`。

## 6. 后台调度器

| 作业 | 频率 | 幂等键 | 说明 |
| --- | --- | --- | --- |
| 月度发放 | 每月 1 日 00:05（时区可配） | `grant:<user>:<yyyy-mm>` | 按用户钱包发放配置额度；失败重试，启动补跑 |
| 钱包 hold 过期 | 每小时 | `hold-expire:<hour>` | 释放过期 `wallet_holds`；失败重试、启动补扫（`22` §4） |
| pending usage 清扫 | 每小时 | `usage-pending:<hour>` | 结算/void 超时 pending usage；失败重试、启动补扫（`22` §5.2） |
| 钱包对账 | 每小时 | `wallet-reconcile:<hour>` | 校验 `balance = Σ账本`、hold ↔ pending；不一致告警/冻结，不自动改账（`22` §6） |
| 月用量快照 | 每月 1 日 02:00（时区可配） | `usage-snapshot:<yyyy-mm>` | 固化不可变月用量快照；失败重试，错过启动补跑（`22` §7） |
| 应用退役 | 每小时 | `app-retire:<app>:<version>` | 软删 → 保留期 → 真删（含 DB 文件） |
| 令牌清理 | 每日 | `token-clean:<date>` | 过期 refresh/黑名单 |
| 用量聚合 | 每小时 | `usage-agg:<hour>` | 生成日报/月报 |
| 审计保留 | 每日 | `audit-retain:<date>` | 按保留策略归档/删除 |
| 客户端版本检查 | 每小时 | `client-ver:<channel>` | 生成更新清单缓存 |

调度器是进程内的：单实例，错过执行在启动时补跑一次（幂等）。不做分布式调度。

## 7. 可观测与运维

- 结构化日志（tracing）+ 请求 ID + 用户 ID（脱敏）；管理台可查最近错误。
- 审计：谁在何时做了什么（管理操作、审批、下发），只追加；**哈希链必选 + WORM** 见 `21` §7。
- 指标：HTTP QPS/延迟、网关上游延迟与错误、余额阻断次数、队列深度、DB 连接池、WASM 执行数。
- `/healthz`（进程活着）、`/readyz`（DB 就绪 + 迁移完成 + 必需包 Active）。
- 备份：`pg_dump` + `/data`（应用 SQLite/日志）；**master.key 单独离线保管，不放进备份包**；恢复流程见 `20` §3。
- 升级：停服 → 备份 → **离线导入镜像（`docker load`）** → 改 `SERVER_IMAGE` → `docker compose up -d` → 迁移自动执行 → `/readyz` 校验 → 失败回滚镜像与备份（见 `09` §7.2，禁止自动拉取/自动升级）。

## 8. 测试

| 层 | 内容 |
| --- | --- |
| 单测 | 每个服务端包：仓储/服务/权限/错误路径 |
| 迁移测试 | 空库 → 全量迁移 → schema 快照比对；旧库 → 新迁移；不可变校验（改历史文件必须红） |
| 路由契约测试 | 生成的全量端点：未授权 401、资源不存在/无权访问 404、角色权限不足 403、必填校验、请求体上限 |
| 集成测试 | 起临时 PG（testcontainers）+ 全包组合，跑认证/网关/钱包/技能/应用主流程 |
| e2e | 真客户端 + 真服务端（docker compose）跑登录、对话、计费、技能安装、应用开窗 |
| 安全 | 认证绕过、CSRF、SSRF（网关上游）、路径穿越（归档解压）、WASM 逃逸/资源耗尽 |

## 9. 明确不做

- 不做 HA/集群/分布式调度；不做多租户；不做服务端 agent 执行；不做运行时包安装；
- 不承诺单进程内插件隔离（WASM 应用是唯一沙箱，见 07 §7）。
