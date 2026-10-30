# 07 · 服务端领域设计（v3.2：弱管控）

> 服务端 = **认证 · 用户 · 钱包/计费 · AI 网关 · 商店（技能/连接器/应用）· 审计 · 管理台 API**。
> 它是**企业版追加的管控包集合**（企业版 = 个人版 + 企业管控包）；个人版不部署服务端。
> **弱管控**：服务端只管"身份、钱、商店内容、记录"，不管员工怎么用客户端；工具默认全开、网络不限制、
> 不做企业审批（`00-design-principles.md`、ADR-030）。
> agent 循环在客户端；服务端不跑 agent、不存会话。单租户、PostgreSQL、docker compose。
>
> 状态：**✅ 已拍板** · **🟡 默认值**（实现前由对应里程碑 spec 确认，索引见 `15` §C） · **⛔ 明确不做**。

## 0. 总口径

### 0.1 做 / 不做

| 做 | ⛔ 不做 |
| --- | --- |
| 认证与会话、用户、钱包与账本、AI 网关与计费、技能/连接器/应用的分发与审批、审计、管理 API、门户与客户端分发 | agent 循环/工具执行、会话存储、工具白名单、网络策略、企业审批、部门/用户组、多租户、部门预算、MCP 流量代理、技能内容执行、应用内 AI、运行时插件加载、LLM 响应缓存 |

**服务端的五个管控点（只有这些）**：

1. **身份**：谁能登录、角色是什么、禁用后立即失效；
2. **钱**：余额、发放、充值、价格、扣费、超额阻断；
3. **商店**：企业技能/连接器/应用的目录、版本、审批、可见性、强制更新与删除；
4. **记录**：审计与用量，可按人/时间/对象查；
5. **构建期**：edition/OEM/渠道在构建时固定，运行期不可切换。

除此之外，员工在客户端里怎么用、用什么工具、访问什么网站，服务端**不管**。

### 0.2 双版本与组合

| 项 | 结论 |
| --- | --- |
| 双版本 | `editions/enterprise.toml` 含 `srv.*` + `admin.*` + 客户端登录/网关客户端；`editions/personal.toml` 只有客户端（无服务端、无登录、无应用中心） |
| 组合 | 服务端也是 cordis 包 + 构建期组合（ADR-021）：`editions/*` + `profiles/*` → `zk compose` → `apps/server/src/generated/` |
| 客户端地址 | 企业版客户端从构建期 `channel.json` 的 `defaults.server_url` 取默认服务端地址；非空 → 登录页不显示地址输入；为空 → 首配页输入并本地保存（设置可改，需重登） |
| 命名 | 包 id `zeokeel/srv-auth-oidc`、crate `zk-srv-auth-oidc`、角色 `srv.auth.source`、目录 `packages/server/auth-oidc/`；管理台 `@zeokeel/admin-users`（crate `zk-admin-users`） |
| 单二进制 | 管理台 dist 由 `admin-*` 包组装 → `include_dir!` 进服务端；门户/客户端安装包随镜像（ADR-025） |
| 唯一稳定缝 | 两端只共享 `zk-contract-*`；领域契约见 §9.1 |
| 运行时 | 无 Redis/MQ/S3；有界进程内队列 + TTL 缓存；单实例不做 HA（`06` §1） |

### 0.3 参考实现教训 → 落点

| 教训 | 落点 |
| --- | --- |
| 路由唯一真源 + 权限点同处申报 | `zk-srv-http` 汇总，包不自建 Router |
| 一套内容模型覆盖多形态 | `contents/content_releases/content_grants/content_deployments`，技能/应用只差 `kind` |
| 网关四段式（准入→转发→计量→结算） | §4；结算与 usage 同事务 |
| 数值单一真源 + 生成物 + 变异测试 | `limits` → `limits.json`/文案/文档 |
| 交付面自包含、渠道构建期注入 | `09`；§10 |
| 迁移可改写/无修复通道（事故） | ADR-022：不可变 + 校验和 + 只前向 |
| 多套并行额度（技术债） | ADR-023：只有一本钱包账 |
| 生命周期只有隐式指针 | ADR-026：`deployment` 表 + 退役作业 |
| WASM 宿主内置 AI（已删除） | 我们的应用**不提供** AI 宿主函数 |

### 0.4 弱管控边界

原则/会做/不会做/责任声明见 [`00` §2–§3](00-design-principles.md)；本节不重复。

## 1. 认证与会话

**口径**：✅ 单一主源（`local` / `OIDC` / `LDAP` 三选一，换源=换包）+ 本地应急超管；✅ 短效 JWT + refresh；
✅ 禁用/降权后**立即失效**；✅ 钉钉/企微/飞书后续为独立包（形态按各家官方，优先级低）。

**规则**：access/refresh 生命周期与吊销语义见 `04` §3（默认值/收敛点：`15` §C T14）；
`token_epoch` 每请求校验（≤5 s 缓存，写操作查库）；切换主源需要迁移/绑定流程（`15` §C T14）。

**接口**：登录/刷新/登出端点见 `04` §2.1/§3；管理台页面见 `08`。用户与角色见 §2，钱包见 §3。

## 2. 用户与角色

**口径**：✅ 角色只有**超管 / 审计员 / 员工**（D14）；✅ 无部门、无用户组、无组织树（D29）；
✅ 禁用/降权后 `token_epoch` 每请求校验、立即失效（D30）；✅ 多设备登录允许、MFA 可选 TOTP（D53）；
✅ 内容可见性 = 全员/指定用户（D47，见 §5/§7）。

**用户来源**：本地创建，或由单一认证源（local/OIDC/LDAP）首次登录建立/同步；审计员只读。

| 表 | 关键列 |
| --- | --- |
| `users` | id, username, display_name, role(superadmin/auditor/employee), status(active/disabled), token_epoch, created_at, last_login_at, deleted_at |
| `user_devices` | id, user_id, device_id, platform, last_seen_at（refresh 令牌按设备存哈希，见 `04` §3） |

- 停用：`status=disabled` + `token_epoch += 1`，会话/刷新令牌全部失效。
- 删除：软删（`deleted_at`），登录拒绝、审计保留且不可改；禁止删除最后一个超管。
- 角色变更写审计；管理台页面/权限见 `08`，客户端 API 见 `04` §2。完整 DDL 在 M2 spec 冻结（`15` §C T7）。

## 3. 钱包与配额边界

**口径**：余额 = 企业配额（D60，非预付资金、不可提现/转让/退款）；一本账（ADR-023）；准入原子 hold；
月发放全员、`accumulate`/`overwrite`（覆盖必须写 `adjust`）；仅管理员分配/调账；峰谷价按请求发生时段 +
价格版本结算；超额硬阻断，流式已输出部分可透支到下限；未定价模型拒绝。

计量、对账、报表、钱审计矩阵与账本/价格表结构见 [`22` §1–§5](22-finance-billing.md)。
**边界**：钱包包只管账户/账本/价格/结算；网关在“准入→转发→计量→结算”四段中调用钱包（§4）；
服务端其它域不得直接改余额列。

## 4. AI 网关

**口径**：✅ 唯一 AI 出口；✅ 只支持 **OpenAI Responses API**（`POST /v1/responses`，流式 SSE + 工具调用透传）；
✅ 上游 provider 与密钥只在服务端；✅ 路由 + 故障转移；✅ 限流；✅ 计量与结算同事务；✅ 未定价拒绝；
✅ 流式超额允许透支；⛔ 不做 Chat Completions、不做响应缓存。

**上游**：`providers` 表（kind/openai-compatible, base_url, api_key 加密, enabled, priority, timeout, max_retries）；
`models` 表（模型名→provider、能力、启用、价格引用）；bootstrap 下发模型目录用于首屏/缓存展示，`GET /v1/models` 提供实时可用性与价格（客户端以 bootstrap 缓存 + 实时刷新组合）。

**API**：`POST /v1/responses`（流式，`stream:true`；工具调用子树不改写）；`GET /v1/models`；
管理面 `GET/POST/PATCH /api/admin/providers`、`GET/PUT /api/admin/models`、`GET /api/admin/usage*`。

**错误**：不向上游细节；`MODEL_NOT_FOUND`、`MODEL_NOT_PRICED`、`WALLET_EXHAUSTED`、`RATE_LIMITED`、
`UPSTREAM`（可重试标记）。

## 5. 技能中心

**口径**：✅ DSH 技能格式（zip + `SKILL.md` + 资源）；✅ 客户端执行，服务端**不执行、不扫描**（扫描记 TODO，见 `15` §C T18）；
✅ 16MB 上限；✅ 员工上传需管理员审批；✅ owner=首发布者，管理员可转交；✅ 可见范围**全员/指定用户**（无部门）；
✅ 新版本**强制更新**；✅ 删除技能 → 已安装客户端**自动同步删除**。

**内容模型**（与 §7 共用）：

| 表 | 关键列 |
| --- | --- |
| `contents` | id, kind(skill/app), name, owner_id, visibility(all/specified_users), status(pending/approved/rejected/delisted), current_release, created_at |
| `content_releases` | id, content_id, version, sha256, archive(bytea), size, changelog, created_by, approved_by, approved_at |
| `content_grants` | content_id, grantee(user/all), can_install |
| `content_deployments` | id, content_id, release_id, action(publish/rollback/delist/delete), actor, at, reason |

**规则**：

- 上传：员工 → `pending`（管理员代传即 `approved`）；同名同版本 409；版本永久占位。
- 审批：管理员 `approve/reject`；拒绝给理由；归属转交；可见范围修改（全员 ↔ 指定用户）。
- 安装/更新：客户端拉清单（版本+sha256）；新版本强制覆盖；**删除时客户端收到事件后移除本地副本**。
- 配额：16MB/技能；每用户上传 50 个、总量 5GB（默认值，M3 spec 确认；`15` §C T10）。
- 扫描：暂不做；`content_releases.scan_status` 预留字段（`15` §C T18）。

## 6. 连接器（MCP）中心

**口径**：✅ 企业连接器由管理员配置下发；✅ 支持 **stdio（本地进程）与 http/sse**；✅ 员工可启用/停用；
✅ 更新强制覆盖；✅ **OAuth 回调与令牌都在客户端**（令牌客户端本地，D53）；个人版允许本地自配（无服务端）。

| 表 | 关键列 |
| --- | --- |
| `connectors` | id, name, transport(stdio/http/sse), config(json), enabled, updated_at；**M3 仅允许无静态 secret 配置（ADR-060）**；静态 secret 加密/受控交付在 M3.1 |
| `connector_grants` | connector_id, grantee(user/all), can_enable |
| `connector_oauth` | connector_id, client_id, scopes, redirect_uri（客户端固定）；**公共客户端 + PKCE，不下发 client_secret**（`21` §3）；OAuth authorize/token endpoint 存 `connectors.config` 的非敏感 `authorize_url`/`token_url` |

- 管理员配置 stdio 命令/参数/环境变量，或 http URL；员工只看到"启用/停用"与状态。
- 员工启用后，客户端执行本地 stdio 或连接远程；OAuth 公共客户端 + PKCE，授权码在客户端完成，令牌本地保存；服务端只存连接器定义/公共客户端元数据（无 client_secret）。
- 更新：管理员改配置 → 客户端强制拉新配置并重启该连接器；审计记录。

## 7. WASM 应用中心

**口径**：✅ 服务端 wasmtime 执行；✅ 每应用一个 SQLite；✅ 应用 UI 为 release 内静态资源，客户端按应用独立 origin 开窗（ADR-062）；
✅ **应用不可调 AI**；✅ **员工可以上传应用**（管理员审核/上架）；✅ 管理员可上架/下架；
✅ 生命周期 App/Release/Deployment + 退役调度。
**语言（已调研，`research/wasm-languages.md`）**：v1 Tier-1 = **Go(wasip1) + Rust(wasip1)**，共用 p1 core module；
TinyGo 预览；C/C++ 仅 vendor；wasip2 与其他语言暂缓。**M4 先 Go 打通，GA 前加 Rust**。

**模型**：复用 §5 的 `contents/releases/grants/deployments`（`kind=app`）+ `app_databases(app_id, schema_version, size_bytes, quota_bytes)`；DB 文件路径由服务端生成、不入库
+ `app_calls`（审计/限额）。

**API**：`GET /api/client/v1/apps`（可见目录）、`POST /api/client/v1/apps/{app_id}/open`、
`POST /api/client/v1/apps/{app_id}/request`（帧 ABI 信封）；管理面/上传面见 `04`。

**运行**：Wasmtime LTS（候选 48/50，M4 实测定版；原型用 50；`15` §C T11）embedding；wasip1 core module；零 preopen；**客户端不执行应用 WASM，只执行 `app_request`**（ADR-062）；
（原型验证见 `experiments/wasm-poc`：Go/Rust 双 guest + 帧 ABI + SQLite + fuel 已跑通）
无网络、无文件；宿主提供 `db.exec/db.query/log/asset.read` 等扁平导入；帧协议 `RS<len>\n<JSON>`；**schema 由 host-managed migration 在 guest 前执行**（release manifest 声明，guest 只 DML）；
epoch/fuel + StoreLimits 做超时/内存；内存/CPU/时间/并发限额；
每应用 SQLite 上限 **100MB / 16 表**；退役作业：软删 → 保留期 → 真删（含 DB 文件）。

## 8. 审计与运维

| 表 | 关键列 |
| --- | --- |
| `audit_log` | id, actor_id, actor_role, action, target_type, target_id, at, result（**哈希链/request_id/device 等完整性字段见 `21` §7**） |
| `usage_daily` | day, user_id, model, requests, tokens, amount_micro |
| `system_settings` | key, value, mode(hot/restart), min_value, updated_by, updated_at |
| `job_runs` | job, key, status, started_at, finished_at, error |

- 审计写入在业务事务内（写失败即业务失败）；审计员只读查询。
- 保留期：审计 ≥180 天、用量 ≥12 个月；**可配置但不得低于下限**（`system_settings.min_value` 校验）。
- 运维：`/healthz`、`/readyz`、备份/恢复、升级/回滚（`06` §7）；**员工客户端下载门户**（服务端提供，非 OEM 自助门户）。
- 系统配置：`hot`（30s TTL）与 `restart` 标注；管理台保存后显示生效状态。

## 9. 服务端包清单

### 9.1 服务端领域契约（本文引用 id；完整目录见 `02` §5）

`zk-contract-api`、`zk-contract-events`、`zk-contract-auth`、`zk-contract-user`、`zk-contract-wallet`、
`zk-contract-gateway`、`zk-contract-content`（服务端技能/应用共用存储）、`zk-contract-connector`。
客户端侧的 `zk-contract-skill`/`app` 是面向客户端的契约，与服务端内部 `content` 聚合不冲突。

### 9.2 服务端域包（清单由 `zk plan` 派生）

| 域 | 包（crate） | 角色 |
| --- | --- | --- |
| auth | `zk-srv-auth-local` / `-oidc` / `-ldap` / `-session` | `srv.auth.source`、`srv.auth.session` |
| users | `zk-srv-users` | `srv.users` |
| wallet | `zk-srv-wallet`、`zk-srv-pricing`、`zk-srv-wallet-grant` | `srv.wallet`、`srv.pricing`、`srv.job.wallet-grant` |
| gateway | `zk-srv-gateway`、`zk-srv-gateway-upstream-openai`、`zk-srv-gateway-meter` | `srv.gateway`、`srv.gateway.upstream`、`srv.gateway.meter` |
| content | `zk-srv-content`（发布内核）、`zk-srv-skills` | `srv.content`、`srv.skills` |
| apps | `zk-srv-apps`、`zk-srv-app-runtime`、`zk-srv-app-db` | `srv.apps`、`srv.app.runtime`、`srv.app.db` |
| updates | `zk-srv-updates` | `srv.updates` |
| connectors | `zk-srv-connectors` | `srv.connectors` |
| ops | `zk-srv-audit`、`zk-srv-settings`、`zk-srv-jobs`、`zk-srv-http` | `srv.audit`、`srv.settings`、`srv.jobs`、`srv.http` |

### 9.3 管理台包（清单由 `zk plan` 派生）

`admin-shell`、`admin-users`、`admin-wallet`、`admin-pricing`、`admin-gateway`、`admin-usage`、
`admin-skills`、`admin-apps`、`admin-connectors`、`admin-audit`（审计员可读）、`admin-settings`、`admin-ui-kit`。

## 10. 与客户端契约的边界

- 客户端→服务端：登录/刷新、bootstrap（**只下发模型目录、商店目录、更新清单、渠道信息**；不下发工具/审批/网络策略）、
  网关请求、技能/连接器/应用目录与内容、用量自查、事件订阅。
- 服务端**不**下发：工具白名单、审批规则、网络策略、员工操作策略。
- 事件：`skill.updated/deleted`、`connector.updated`、`app.listed/delisted/version`、`config.updated`、
  `model_catalog.updated`、`balance.updated/low`、`update.required`、`session.revoked`。
- 详情见 `04-contracts-and-api.md`。

## 11. 决策与默认值（本文件编号 07-Q*；Q1/Q4 为默认值）

| # | 事项 | 结论 |
| --- | --- | --- |
| 07-Q1 | 应急超管来源限制 | 仅本机/内网（默认值） |
| 07-Q3 | 月发放单人覆盖 | 当前不做（全局配置） |
| 07-Q4 | 技能配额 | 50 个/人、总量 5GB（默认值） |
| 07-Q10 | 流式透支 | 记欠费，下次阻断 |

以上均为默认值；开放项索引见 `15` §C T14。
