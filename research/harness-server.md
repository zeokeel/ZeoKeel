# ZeoKeel Harness 服务端事实简报

> 调研对象：`/tmp/zk-harness/server`（Go 1.26.6 单二进制；1024 个 `.go`、约 32.4 万行）。
> 只读调研，2026-10-30。证据写作 `文件:行号`；`internal/` 省略为 `server/internal/`，`migrations-pg/` 省略为 `mig/`。
> 文档与代码冲突处以**代码**为准，冲突单独标「文档漂移」。

## 0. 一页速览

| 维度 | 事实 | 证据 |
|---|---|---|
| HTTP | gin v1.12 + net/http；路由**唯一真源** `internal/router`（业务包不得自建 Group） | `server/go.mod:7`；`server/internal/router/router.go:1-14` |
| DB | PostgreSQL only（pgx v5），82 条内嵌 SQL 迁移 `0001–0090`，启动即迁移 | `server/go.mod:10`；`server/internal/serverstore/migrate.go:22-23,147` |
| 部署 | 3 容器：caddy + server + postgres；webadmin **编进** server（`go:embed`），无独立容器 | `server/docker-compose.yml:44-50,77-83,208-232`；`server/webadmin/embed.go:12` |
| 员工认证 | 不透明 Bearer token（32B 随机，库里只存 sha256），90 天，无 JWT/无 refresh | `server/internal/serverauth/token.go:14-15` |
| 管理员 | Cookie `zeokeel_session`（HttpOnly）+ CSRF，12h 硬上限 + 60min 空闲；TOTP MFA | `server/internal/serverauth/admin.go:29`；`admin_session.go:17-25`；`mfa.go:19-25` |
| 部门 | `groups` 自引用树 + `user_groups`；有效组 = 归属+祖先+主管子树+「全员」 | `server/internal/serverstore/effective.go:112-160` |
| 额度 | **唯一闸门 = 员工账户余额**（元/微元）；token 配额、金额配额、部门预算**已下线但列仍在** | `server/internal/serverstore/balance.go:15-35`；`llmgateway/balance_gate.go:1-45` |
| 网关 | OpenAI + Anthropic 兼容，多 provider 故障转移，计量与扣费**同事务** | `server/internal/llmgateway/handler.go:119-215` |
| 内容模型 | 技能 / Agent / WASM 应用共用 `apps` + `app_releases` + `app_grants` 三表 | `server/internal/serverstore/migrations-pg/0053_apps.sql` |
| WASM | wazero + `wasm32-wasip1`，帧协议 ABI，**每应用一个 SQLite**，客户端专属协议打开 | `server/internal/wasmapp/runtime/runtime.go:17-30`；`abi/abi.go:1-20` |

---

## 1. 整体架构

### 1.1 进程模型与路由组织

| 项 | 事实 | 证据 |
|---|---|---|
| 进程 | 单进程：认证 + 网关代理 + 内容分发 + 计量 + 管理端静态资源；密钥只在服务端 | `server/docs/01-architecture.md:20-26` |
| 命名空间 | `/api/server`（管理面，**无版本号**）、`/api/client/v2`（员工面，v2 即大版本）、`/v1`（LLM 网关） | `router.go:40-44` |
| 集中注册 | `Register()` 一次装配全部路由；路径+方法+中间件+RBAC 权限点同处申报，可枚举可测试 | `router.go:88-93`；`router.go:314-444`（管理面逐条 `AdminRoute(..., PermXxx, h)`） |
| 网关挂载 | `/v1/*` 与**无 `/v1` 前缀**的官方原生变体双挂；中间件顺序 `BearerAuth → InFlightGuard`（准入需已认证用户） | `router.go:275-311` |
| 文件面 | 安装包下载挂根路径 `/updates/client/*`（大文件语义，脱离 `/api` 强制 JSON 契约） | `router.go:107-108` |
| 可选模块 | `skills` / `apps` / `connectors` 用 build tag **编译期裁剪**；关闭时整片路由与 handler 树从产物消失（而非注册 nil） | `router.go:28-31`；`router/optional_apps_on.go:26-38`；`router/optional_skills_on.go:1-10` |
| 请求体上限 | 两个业务命名空间统一 1 MiB，白名单路由（归档 24 MiB / wasm 发布 48 MiB / 分片 8 MiB）自套 `MaxBytesReader` | `router.go:136-192` |

### 1.2 `internal/` 域划分（30 个包，按职责）

| 域 | 包 | 说明 |
|---|---|---|
| 入口/装配 | `cmd/server` | 迁移、调度器装配、bootstrap-admin、`--reset-mfa` 等运维模式 |
| 路由 | `router` | 唯一路由真源（含 wasm app_id 保留字派生） |
| 认证/管理 | `serverauth` | local/LDAP/OIDC、token、admin session、MFA、RBAC、部门/用户/审计 handler |
| 存储 | `serverstore` | 全部 SQL、迁移、分区、账本、授权、分发语义 |
| 网关 | `llmgateway` | 兼容层、上游路由、限流、计量、文件台账、余额闸门 |
| 内容 | `appstore` `sharedskills` `agentshare` `marketplace` `capabilities` `skillmanifest` `skillseed` `archiveutil` | 统一发布内核 + 各渠道面 + 包规范 + 内置技能 |
| WASM | `wasmapp/*`（29 子包） | runtime / abi / appdb / applimits / compile / api / queue / retirement / opens / diag |
| 其他 | `balance` `reports` `connectors` `channel` `portal` `clientrelease` `updatecheck` `deployconfig` `features` `limits` `telemetry` `auditchain` `auditretention` `usageretention` `tokenretention` `bootstrap` | 发放调度、月报、连接器、渠道、门户、安装包、版本检查、配置注册表、限额真源… |

### 1.3 数据库

| 项 | 事实 | 证据 |
|---|---|---|
| 类型 | PostgreSQL（SQLite 已于 2026-08 全面下线；`modernc.org/sqlite` 仅用于 WASM 应用库） | `migrate.go:147`；`server/go.mod:15` |
| 迁移方式 | `//go:embed mig/*.sql` 内嵌 → 启动时按 version 升序执行；`schema_migrations(version, applied_at, checksum)` | `migrate.go:22-23`；`server/docs/06-database.md`「迁移执行器的运维口径」 |
| 规模 | 82 个迁移文件（0001–0090，其中 0007 废弃、0028 下线旧表），45 条 `CREATE TABLE` | `ls server/internal/serverstore/migrations-pg` |
| 分区 | `usage` 按 `created_at` 月分区；`usage_daily` 按年分区；账本永久保留 | `mig/0039_usage_partition_ledger.sql` |
| 连接池 | `database/sql` + pgx；有"持事务再向池要连接"死锁事故记录，读面收口为 `*Q(q)` 形态 | `serverstore/gateway.go:1085-1125` 注释 |

核心表（按域）：

| 域 | 表 | 关键列/语义 |
|---|---|---|
| 身份 | `users` | `role`(super_admin/auditor/user)、`source`(local/ldap/oidc)、`external_id`+`external_source`、`password_must_change`、`totp_secret/totp_enabled`、`balance_money`、`balance_activated_at` |
| 令牌/会话 | `api_tokens`、`admin_sessions` | 只存哈希（`token_hash` / `secret_hash`）；session 带 `csrf_key`、`last_used_at` |
| 组织 | `groups`、`user_groups` | `parent_id` 任意层级、`leader_id`；`budget_money` 为**死列** |
| 配置 | `settings(key,value)` | 全部运行期开关的 KV（30s TTL 进程内缓存） |
| 网关 | `gateway_providers`、`models` | `api_key_enc` AES-GCM；`protocol`(openai/anthropic/both)；模型价目列 |
| 计量 | `usage`(分区)、`usage_daily`、`usage_monthly` | `cost` 记录时定价；`kind`(chat/embedding/search)；`app_id` 归因 |
| 资金 | `balance_ledger`、`balance_grant_items` | 追加型流水；不变量 `users.balance_money == SUM(ledger.amount)`；人·月发放幂等锚 |
| 内容 | `apps`、`app_releases`、`app_grants` | 长期身份 / 不可变版本（`archive BYTEA`）/ 用户或部门授权 |
| 其他 | `connectors`、`gateway_files`、`audit_logs`(哈希链)、`report_subscriptions`、`wasm_app_deployments`、`wasm_app_limits`、`wasm_call_events`、`wasm_app_opens(_daily)` | — |

### 1.4 缓存 / 队列 / 对象存储 / 后台任务

| 能力 | 实现 | 证据 |
|---|---|---|
| 缓存 | **无 Redis**；进程内 TTL 缓存（settings 30s、上游列表、模型价目、限流桶）+ wazero 磁盘编译缓存 + 模块缓存（128 MiB/128 条，空闲 10min 淘汰） | `serverstore/settings.go:61`；`llmgateway/upstream.go:46-83`；`limits/limits.go:186-197` |
| 队列 | **无 MQ**；全部进程内有界队列：每应用请求队列 32（满 429）、编译队列 64/并发 1、单用户并发 32 | `limits/limits.go:120-122,400`；`llmgateway` `InFlightGuard` |
| 对象存储 | **无 S3**：技能/Agent/WASM 归档直存 PG `BYTEA`；WASM 静态资源与 `app.db` 落盘 `data/apps/<app_id>/`；`data/` 0700 | `mig/0053_apps.sql`；`limits/limits.go:225` |
| 后台任务 | 10+ 个可观测调度器（表内 7 个 + 三条清理执行者 + 2 条接缝）：reports、balance、model_sync、directory_sync、audit_retention、usage_retention、token_retention、gateway_files reaper、wasm_retirement、wasm_opens | `cmd/server/scheduler_status.go:64-85` |

### 1.5 配置来源（优先级从高到低）

| 层 | 内容 | 证据 |
|---|---|---|
| 环境变量 | 仅 4 个引导键（DOMAIN/TLS_MODE/ADMIN_PASSWORD/PG_PASSWORD）+ "有意不入库"的部署旋钮（网络/连接池/迁移预算/镜像路径/master key） | `server/docker-compose.yml:7-24` |
| `settings` 表 | 控制台「系统配置」「网关」「认证」等页面落库的键 | `serverstore/settings.go` |
| 注册表默认 | `deployconfig` 12 键（登录失败上限、OIDC 信任主机、并发上限、遥测预算、审计链间隔、应用保留名、内存档位、编译隔离…） | `deployconfig/keys.go:107-240` |
| 镜像文件 | 渠道内容 `/opt/zeokeel/channel/`、内置技能 `/opt/zeokeel/skills/`、客户端包 `/opt/zeokeel/client/`、`VERSION`/`CHANNEL` | `channel/channel.go:1-30`；`server/Dockerfile:205-230` |

优先级 = **非空 env > settings 保存值 > 内置默认**；`restart` 类键需重建容器（`server/docs/02-build-deploy.md:52-63`）。

### 1.6 部署形态（docker-compose）

| 服务 | 镜像 | 网络/端口 | 职责 |
|---|---|---|---|
| `caddy` | `caddy:2-alpine` | 80/443 对外，唯一入口 | TLS 终结 + 反代；`TLS_MODE=manual/auto/internal` 选三份 Caddyfile 之一 |
| `server` | `${SERVER_IMAGE}` | 仅 `expose 8080`，不发布宿主端口，固定 IP 172.28.0.3 | Go 服务端（含内嵌 webadmin、门户、安装包、技能） |
| `postgres` | `postgres:18-alpine` | 无宿主端口，固定 IP 172.28.0.4 | 业务库 |

证据：`server/docker-compose.yml:44-76`（caddy）、`:77-207`（server）、`:208-232`（postgres）。**没有 webadmin 容器**——`webadmin/dist` 由 `go:embed` 进二进制，走 `/admin/`。

---

## 2. 用户认证

### 2.1 三种密码模式 + 可选 OIDC

| 模式 | 行为 | 端点 | 证据 |
|---|---|---|---|
| `local`（默认） | 本地 `users` 表，argon2id 校验 | `POST /api/client/v2/auth/login` | `serverauth/local.go:24-40` |
| `ldap` | 仅 LDAP bind；用户不存在则建档 | 同端点 | `serverauth/ldap.go:360-425` |
| `both` | 先 local 后 ldap 逐个尝试 | 同端点 | `serverauth/config.go`（`auth.mode`） |
| OIDC / OpenID | 浏览器授权码 + PKCE；**两条路由固定注册**，provider 请求时解析（改配置即生效，不重启） | `GET /auth/{oidc,openid}/login`、`/callback` | `router.go:222-229`；`oidc.go:125-160` |

OIDC 细节：

| 环节 | 事实 | 证据 |
|---|---|---|
| Scope | `openid profile email`（groups 靠 IdP 额外下发） | `oidc.go:147` |
| 用户映射 | `preferred_username` → 空则 `email` → 空则 `sub` | `oidc.go:255-262` |
| 稳定主体 | 绑定 `sub`（`external_id`）；已绑定的行不接受不同 `sub` 接管 | `oidc.go:266-279`；`handler.go:772-782` |
| 组映射 | `groups` claim → 部门（**指针区分"未下发"与"空数组"**，未下发时不清空） | `oidc.go:249,264-274`；`handler.go:783-789` |
| CSRF/重放 | `state` 同时写 SameSite=Lax cookie，回调必须回显；nonce 常量时间比对 | `oidc.go:283-289,440-462` |
| 回跳 | 302 到渠道深链 `<scheme>://auth?token=…&server=…&user=…` | `oidc.go:519-535` |
| 审计 | 成功/失败各写 `login_success`/`login_fail` | `oidc.go:490,530` |

LDAP 细节：`ldap.server_url/bind_dn/bind_password/base_dn/user_filter/group_filter/group_attr`；分页 200；`group_filter` 是否参与组同步由**唯一谓词** `syncsGroups()` 决定（登录与同步两条路径共用），缺失时不清空已有组（`ldap.go:141,200-220,330-334`）。明文 LDAP 受 `ZEOKEEL_LDAP_ALLOW_PLAINTEXT` 闸门（`deployconfig/keys.go:142`）。

### 2.2 会话机制（无 JWT）

| 项 | 员工面 | 管理面 |
|---|---|---|
| 载体 | `Authorization: Bearer <token>` | Cookie `zeokeel_session`（HttpOnly、SameSite=Lax、Secure 按部署判定） |
| 服务端存储 | `api_tokens.token_hash = sha256(token)` | `admin_sessions.secret_hash`（明文 session id 不落库） |
| 生命周期 | **90 天**，`expires_at NOT NULL`；可多 token 并存（name 默认 `desktop`） | **12h 硬上限** + 60min 空闲滑动过期 |
| 吊销 | `POST /auth/logout` 置 `revoked`；改密/降权/禁用**同事务**吊销全部 token 与会话 | `POST /admin/logout` 删行；改密吊销全部 |
| CSRF | 无（Bearer） | 写请求必须 `X-CSRF-Token`；token = HMAC(csrf_key, "session:"+sessionID)，与会话同寿命 |
| 刷新 | **无 refresh token**，过期即重新登录 | 滑动续期（空闲窗口内） |

证据：`token.go:14-15,36`；`admin_session.go:17-25,55-70,162-185`；`admin.go:518-535`。

### 2.3 MFA（仅管理员，TOTP）

| 项 | 事实 | 证据 |
|---|---|---|
| 开启 | `POST /admin/me/mfa/enable {password}` → 下发 `secret`+`otpauth_url`+`ticket`；60s 内 `verify` | `mfa.go:25`；`server/docs/03-api-reference.md:62-66` |
| 两步登录 | 密码通过后不建会话，返回 `{mfa_required, mfa_ticket}`；`POST /admin/login/mfa {ticket, code}` | `admin.go:392-457` |
| 挑战 | 一次性、5 分钟；失败 ≥5 次作废；`attempts+1` 与判定在**同一条 UPDATE**（防并发穿透） | `mfa.go:23,126-140` |
| 已开启保护 | 再次 enable ⇒ 409 `MFA_ALREADY_ENABLED`；disable 需主密码+动态码双验 | `server/docs/03-api-reference.md:66-70` |
| 存储 | `users.totp_secret` 为 AES-GCM 密文；TOTP 防重放 `users.last_totp_step` | `server/docs/06-database.md` users 表 |

### 2.4 管理员账号初始化与登录限制

| 项 | 事实 | 证据 |
|---|---|---|
| 引导 | 仅当**不存在任何 super_admin** 时用 `--bootstrap-admin <name>` + `ZEOKEEL_ADMIN_PASSWORD` 建档；密码缺失/太短直接启动失败；同名非管理员 ⇒ 启动失败（防提权） | `bootstrap_admin.go:19-50` |
| 后台登录 | **仅本地账号**（`source=local` 且有 password_hash）；`user` 角色 403；`auditor` 可登录后台但**不得登录客户端** | `admin.go:375-392,598-600`；`oidc.go:504-507` |
| 登录限流 | 失败计数滑动窗口 5 分钟：账号键 10 次（`u:<name>` 与 `ip|name` 共用预算）、来源 IP 60 次；阈值来自 `deployconfig`（文件系统配置可改，保存即生效） | `ratelimit.go:20-56,85,145` |
| 强制改密 | `password_must_change=1` 时除改密/me/logout 一律 403 `PASSWORD_CHANGE_REQUIRED`（BearerAuth 内守卫） | `handler.go:342-352` |

---

## 3. 部门管理

| 项 | 事实 | 证据 |
|---|---|---|
| 数据模型 | `groups(id, name 唯一 NOCASE, parent_id(0=顶层), leader_id, description, budget_money 死列)`；`user_groups(user_id, group_id)` 复合主键 | `migrations-pg/0001_init.sql`、`0017`；`serverstore/departments.go:23-48` |
| 层级 | 任意深度自引用树；`parent_id=0` 为顶层；删除要求无成员/无子部门/无授权引用 | `server/docs/03-api-reference.md:96-99` |
| 多部门 | 用户可属多个部门（`PUT /users/:id/department {group_ids:[]}`，兼容旧 `{group_id}`） | `router.go:342`；`server/docs/03-api-reference.md:93-95` |
| 有效权限组 | 归属部门 + **祖先链** + 主管（`leader_id=该用户`）的**子树** + 隐式「全员」组 | `effective.go:112-160` |
| 授权继承 | 授权对象是组名：授权给父部门 ⇒ 子树成员可见；主管自动获得所辖子树授权 | 同上 |
| 隐式全员 | 常量「全员」对所有人生效；不可作为普通组创建/删除 | `effective.go:112-114`；`groups.go:31-33` |

**与认证源同步（谁是真源）**

| 源 | 语义 | 证据 |
|---|---|---|
| LDAP | 目录是**组名真源**：`group_filter` 按用户 DN 查组 → 全量替换本地成员；目录消失的用户**自动停用 + 吊销全部 token** | `dirsync.go:123-215,311-350` |
| LDAP 调度 | 保存配置即触发一轮，此后**每 1 小时**一轮；空目录（0 用户）拒绝执行（防误停全员） | `dirsync.go:54`；`server/docs/03-api-reference.md:97` |
| OIDC | 仅当 IdP 下发 `groups` claim 才同步；缺失声明**不清空**（否则会误删 LDAP 组） | `handler.go:783-789` |
| 组落地 | `SyncUserGroups` **按名字 GetOrCreate**：LDAP/OIDC 的组名自动建成**平铺**部门（`INSERT INTO groups(name)`，无 parent），不做 DN→树映射 | `groups.go:30-55,111-135` |
| 单向性 | 同步只自动**停用**，**永不自动启用**；跳过与停用都写审计（`directory_enable_skipped` / `directory_user_disabled`） | `dirsync.go:289-350`；`server/docs/03-api-reference.md:99` |
| 防接管 | 外部身份不得接管同名**本地**账号（`source != external` 直接跳过/拒绝） | `handler.go:768-771` |

**部门管理员权限：不存在。** RBAC 只有三个全局角色，权限点是 `<域>:<动作>` 常量表，不落库（`rbac.go:19-106`）。部门 `leader_id` 只影响**内容授权继承**（`effective.go:121-127`），不带来任何管理端权限；管理面写操作一律要求 `dept:write`/`user:write` 等全局权限（`router.go:342-347`）。

---

## 4. 预算 / 额度 / 计费

### 4.1 表结构与粒度

| 对象 | 表/列 | 粒度 | 证据 |
|---|---|---|---|
| 账户余额 | `users.balance_money`（微元精度存储、分位展示）、`users.balance_activated_at` | **仅个人**（无部门账户、无共享池） | `balance.go:15-35,163-172` |
| 流水 | `balance_ledger(kind, amount, balance_after, reason, actor, usage_id, month)`，`kind ∈ grant/reset/adjust/consume/refund` | 逐笔追加，不变量 `balance == SUM(amount)` | `balance.go:54-60,197-215` |
| 月度发放锚 | `balance_grant_items(user_id, month)` | 人·月幂等（新入职/漏发自动补齐） | `balance.go:24-30`；`balance/scheduler.go:1-13` |
| 价目 | `models.input_price_per_1m / output_price_per_1m / cache_input_price_per_1m / offpeak_discount` | **按 (provider, model)** 计价 | `serverstore/gateway.go:932-960` |
| 计量 | `usage.cost`（元）、`prompt/completion/cache_prompt_tokens`、`search_requests`、`kind`、`provider_id`、`app_id` | 每次调用一行 | `migrations-pg/0004,0022,0029,0030,0065,0076,0085` |

### 4.2 扣减顺序与闸门（全部在**转发之前**）

| 序 | 判据 | 结果 | 证据 |
|---|---|---|---|
| 0 | 未开通余额账户（`balance_activated_at` 为空） | 不拦、不扣（存量部署开闸不误拦全员） | `balance.go:18-23` |
| 1 | 分位余额 ≤ 0 且 `balance.enabled` | 429 `BALANCE_EXHAUSTED`（admin 豁免） | `balance_gate.go:30-32` |
| 2 | 学到的下限：上次结算因余额不足失败 ⇒ 该账号在余额增长前一律拒 | 429 `BALANCE_EXHAUSTED` | `balance_gate.go:22-26` |
| 3 | 未定价模型（输入价 NULL/≤0） | 429 `MODEL_NOT_PRICED`，缺省策略 `reject`（`allow` 可放行） | `balance_gate.go:27-32` |
| 4 | 最小计费额：估算 prompt token × 输入价 > 余额 | 429 `BALANCE_EXHAUSTED` | `balance_gate.go:20-22` |
| 5 | 无候选 provider（模型名错/全停用） | 不判钱，交 handler 404 | `balance_gate.go:34-38` |

结算：写 `usage` 与扣 `balance_ledger` **同一事务**（`settleUsageCostTx`）。非流式余额不足 ⇒ 整笔回滚（未交付不欠款）；**流式** 因正文已 Flush 给客户端，允许透支（`allowOverdraft=true`，余额走负、欠款入账，后续请求被闸门拦）（`balance.go:236-315`）。

### 4.3 超额策略与费用公式

- 策略只有 **阻断**（429）+ 管理端提示，**没有审批流、没有告警阈值**；运营靠 webadmin 余额页/流水。
- `cost = (未命中输入×input/1e6 + 命中输入×cache/1e6 + 输出×output/1e6) × 峰谷系数`；峰谷窗口 `settings usage.peak_windows`（北京时间），窗口外 × `offpeak_discount`（`serverstore/usage.go:309-323`）。
- `web_search` 按次费用**独立价目**：`search_requests × settings usage.search_price_per_1k / 1000`，**不参与峰谷折扣**（`mig/0085`；`usage.go:341-349`）。
- 改价/改窗口只影响之后产生的费用（记录时定价），历史 `cost` 不重写（`usage.go:328-329`）。

### 4.4 价格表来源与维护

| 环节 | 事实 | 证据 |
|---|---|---|
| 上游同步 | `POST /admin/providers/:id/sync`、`sync-all` 拉 `/models` **只同步模型名/能力**，价格不自动落 | `llmgateway/sync.go`；`channels/channel.go:13-30` |
| 价格录入 | 管理员逐模型手工填（元/百万 token），留空 = 未定价 | `router.go:387-390` |
| 渠道预设 | `channels` 注册表按厂商提供 `RequestOverrides`（请求体深合并/删键）与默认上下文长度，不提供价格 | `channels/channel.go:15-40`；`channels/deepseek.go` |
| 未定价兜底 | 默认拒绝调用（见闸门 3），需显式 `unpriced_model_policy=allow` | `balance_gate.go:27-32` |

### 4.5 账单/报表接口与网关耦合点

| 能力 | 端点 | 证据 |
|---|---|---|
| 员工自查 | `GET /api/client/v2/auth/usage`（余额/是否开通/今日/昨日/本月/累计） | `router.go:210`；`handler.go:821-860` |
| 管理汇总 | `GET /admin/usage`、`/usage/overview`、`/usage/requests` | `router.go:355-358` |
| 资金流水 | `GET /admin/users/:id/balance/ledger`；`POST /users/:id/balance`；`PUT /balance`；`POST /balance/grant` | `router.go:350-354` |
| 月报推送 | `report_subscriptions` + `POST /admin/report-subscriptions/:id/test`（webhook，advisory lock 防多实例重复投递） | `router.go:428-432`；`mig/0082` |
| 与网关耦合 | ①准入闸门在 `ChatCompletions` 内、转发前；②`usage` 与 `balance_ledger` 同事务；③准入估算与结算兜底**同一实现**（`estimatePromptTokensFromBody`）；④流式先插 pending usage 行，失败即删 | `llmgateway/handler.go:113-118,878-884` |

---

## 5. AI 网关

| 项 | 事实 | 证据 |
|---|---|---|
| 对外协议 | OpenAI 兼容（`/v1/chat/completions`、`/completions`、`/responses`、`/embeddings`、`/models`）+ Anthropic Messages（`/v1/messages`）+ DeepSeek Files API | `router.go:275-311` |
| 已知差异 | 未实现 `/beta/*`、`/anthropic/v1/*`、`GET /user/balance`；官方 429=并发上限，本平台 429=余额/本地限流 | `router.go:258-272` 注释 |
| 流式 | SSE 原样转发；强制向上游索要 `stream_options.include_usage`（否则 pending 行无法回填）；逐行/空闲/总时长预算；中断映射为 SSE error 帧而非静默 | `handler.go:176-190,800-860` |
| 工具调用 | 纯透传；但 `tools/functions/tool_choice/response_format` 子树**不参与** `file_id` 收集与私有字段剔除 | `server/docs/03-api-reference.md:123` |
| 上游/provider | `gateway_providers(base_url, api_key_enc, models JSON, protocol, enabled, channel)`；`protocol` 决定同模型由谁来服务；密钥 AES-GCM（`enc:v1:` + master key 文件） | `mig/0003`；`serverstore/gateway.go:895-960` |
| 路由 | `MatchModelsByProtocol(model, protocol)` → 候选列表；按渠道 override 逐候选独立计算请求体（防污染） | `llmgateway/upstream.go:220+`；`handler.go:158-205` |
| 重试 | **故障转移**：连接失败/5xx/首字节超时 → 下一个 provider；**同一 provider 不重试**（防重复计费）；4xx 原样返回 | `handler.go:145-155,205-215` |
| 限流 | per-user 令牌桶 `gateway.rate_limit`（缺省 **0=不限**，老库保留 60 需手改）+ 单用户在飞 32（`InFlightGuard`） | `server/docs/04-auth.md:74-90`；`router.go:280` |
| 审计 | **不逐调用写 audit_logs**；留痕靠 `usage`（含 provider/app 归因）+ 进程内计数 + 结构日志（按用户每分钟节流） | `balance_gate.go:52-60` |
| token 计量 | 优先上游 `usage`，缺失则按字节估算并标 `usage.estimated`；流式在最后一个 chunk 回填 | `usage.go:334-349`；`handler.go:1045-1070` |
| 缓存 | **无 LLM 响应缓存、无语义缓存**；只有静态资源与编译缓存 | `server/docs/01-architecture.md:20-26`；`limits/limits.go:186-220` |
| 失败处理 | 全 provider 失败 ⇒ 删 pending usage + 502 `UPSTREAM`（不回显上游细节）；请求体非法 JSON 一律本地 400（fail-closed，绝不原样转发） | `handler.go:205-215`；`server/docs/03-api-reference.md:150` |
| 出站加工 | 注入官方用户标识 `u<users.id>`（覆盖客户端自带值，防 KVCache 投毒/隔离逃逸）；校验所有 `file_id` 归属（非本人 404，整条不发上游） | `server/docs/03-api-reference.md:121-123` |
| 文件台账 | `gateway_files` 记 `(file_id,user_id,expires_at,size)`；认领+租约+世代号（fencing）回收，绝不删无凭据对象 | `mig/0077-0081`；`llmgateway/files_reaper.go` |

---

## 6. 技能中心

| 项 | 事实 | 证据 |
|---|---|---|
| 格式 | zip（也兼容 tar.gz）根目录必须有 `SKILL.md`；frontmatter **严格校验**：`name`(kebab-case，即运行时身份)/`version`(严格 semver)/`title`/`description`/`author`/`category`，扁平映射、禁 merge key、禁嵌套 | `skillmanifest/manifest.go:1-60,102-300` |
| 包安全 | 原始 ≤16 MiB、解包 ≤64 MiB、条目 ≤10000；拒绝越界路径/链接文件/无根 `SKILL.md`；前端溯源块 `zeokeel`/`.zeokeel/` 视为伪造 | `archiveutil/archive.go:29-70` |
| 存储 | 归档**直存 PG BYTEA**（`app_releases.archive`）；旧磁盘行只读回退 | `mig/0053`；`server/docs/06-database.md` apps 一节 |
| 上传/审批 | 员工 `POST /api/client/v2/shared-skills` → `pending`；管理员 `/admin/shared-skills/:name/:version/{approve,reject}`；**管理员代传即 approved** | `router/optional_skills_on.go:88-100`；`appstore/publish.go` |
| 归属 | `apps.owner` = **首个成功发布者**，一经写入不被覆盖；只有 owner 或管理员能发新版；归属可转移（`PUT /admin/apps/:kind/:app_id/owner`，审计） | `mig/0053`；`router.go:416` |
| 可见范围 | 员工面 = `approved ∧ 已上架 ∧ (已授权 ∨ owner 本人)`；未授权一律 404（与不存在同形）；admin 恒全量 | `serverstore/distribution.go`；`server/docs/03-api-reference.md:236-237` |
| 强制更新 | **没有强制更新**。客户端拿到新版本后显示「更新」，安装 = 整树替换 + 校验 `X-Skill-Checksum`（sha256）；同版本不可覆盖 | `server/docs/03-api-reference.md:308-313` |
| 版本/回滚 | 版本号**永久占位**（rejected/软删/撤回都不复用）；保留「最近 3 个曾生效版本」；回滚 = 装回旧 approved 版本（技能无 deployment 概念） | `limits/limits.go:25`；`server/docs/03-api-reference.md:20` |
| 配额 | 每作者待审 ≤10（429 `PENDING_LIMIT`）；每用户制品总量 1 GiB（含全部版本） | `serverstore/apps.go:916-922`；`limits/limits.go:27` |
| 可执行代码 | 技能是 **Markdown + 随包脚本/引用**：服务端**不执行、不扫描**（无沙箱、无 AV、无签名）；信任来自"管理员人工审核 + 结构校验"，执行发生在员工客户端 agent 运行时 | `skillmanifest/manifest.go:1-10`；`sharedskills/routes.go:296`（只做清单校验，无执行/扫描路径） |

内置技能（另一条通道）：镜像内 `/opt/zeokeel/skills/<name>/`，`GET /api/client/v2/skills/builtin` 清单 + `:name/archive` tar.gz（带 sha256/version 头）；**无授权门**，客户端按需安装、不自动装；目录名必须等于 frontmatter `name`，否则静默跳过（管理端只读诊断面 `GET /admin/skills/builtin` 专门回答"为什么少了"）——`router/optional_skills_on.go:60-142`；`skillseed/skillseed.go`。

---

## 7. 连接器（MCP）

| 项 | 事实 | 证据 |
|---|---|---|
| 模型 | 唯一目录表 `connectors(id, name, description, auth_mode, definition JSON, enabled, updated_at)`；`auth_mode ∈ oauth/device/token/server-side` | `mig/0042_connectors.sql:24-33` |
| 定义结构 | `definition` = `{auth:{discoveryUrl/authorizeUrl/tokenUrl/clientId/redirectUri/pkce/scopes}, tokenFields:[{key,label,type,required}], mcp:[{serverName,transport,url|command,args,env}], examples[]}` | `mig/0042_connectors.sql:44-70` |
| 下发 | 仅 `enabled=1` 进 `GET /api/client/v2/config/bootstrap` 的 `connectors[]`（下架即不下发=客户端隐藏） | `bootstrap/bootstrap.go:194-213` |
| 企业 vs 个人 | **只有企业下发**：员工不能在服务端建个人连接器；个人侧只填凭据（`tokenFields`） | `connectors/admin.go:22-140`（管理端点仅 admin） |
| 密钥位置 | **不在服务端**：服务端只存定义，MCP 的 OAuth token / API key 存在**客户端本地凭据库**（按 `<dshHome>/users/<name>/connectors/`），服务端全程看不到 | `docs/decisions/2026-09-14-connector-token-refresh.md:1-40` |
| 传输类型 | `streamable-http`（远程，OAuth 2.1+PKCE）与 `stdio`（本地子进程，`npx …`，令牌注进 env） | `mig/0042`（种子三行） |
| 启停 | `PUT /admin/connectors/:id/enabled`（上/下架，审计） | `router/optional_connectors_on.go`；`connectors/admin.go` |
| 强制更新 | **定义随每次 bootstrap 全量下发**（服务端是唯一真源），客户端不做本地编辑 ⇒ 改定义即"强制生效"；凭据刷新由客户端按 `expires_in` 提前 60s 续期，失败置 `unauthorized` 并明示需重新授权 | `bootstrap.go:194-213`；`2026-09-14-connector-token-refresh.md:44-60` |
| 安全限制 | 定义 JSON 有体积上限；OAuth discovery 走出站护栏（防 SSRF/DNS rebinding）；服务端不代理 MCP 流量（客户端直连） | `connectors/admin.go:18-20`；`channels/channel.go:57-62` |

---

## 8. WASM 应用中心

### 8.1 运行时与 ABI

| 项 | 事实 | 证据 |
|---|---|---|
| 运行时 | `wazero v1.12`（纯 Go，无 cgo）；目标 `wasm32-wasip1`（Tier1 语言 = Go） | `server/go.mod:12`；`docs/planning/2026-09-17-wasm-app-platform.md:501-510` |
| 必须显式配置 | `WithCloseOnContextDone(true)`（否则 ctx 超时完全不生效）、`WithRandSource(rand.Reader)`（默认是**固定种子 42**）、真实 walltime/nanotime/nanosleep、**零 preopen**、不传 args/env | `2026-09-17-wasm-app-platform.md:147-168` |
| 实例策略 | **只缓存编译结果，每请求新实例**（禁止复用） | 同上 4.3 |
| 内存 | 1 实例 64 MiB（1024 页）；全局 32 实例；档位 default/small/large 由 `ZEOKEEL_WASM_MEMORY_PROFILE` 决定；理论峰值 > 可用内存 70% ⇒ **拒绝启动** | `limits/limits.go:167-220` |
| ABI 帧 | `RS(0x1e) + 十进制长度 + '\n' + JSON`，必须**一次读满**；`abi = "zeokeel-app/1"` | `wasmapp/abi/abi.go:1-20,50` |
| 请求帧 | `{abi, app_id, version, auth:{mode,verified}, user:{id,username,display_name,dept,is_publisher}, method,path,query,headers,body}` — 身份**只**来自宿主构造的帧 | `2026-09-17-wasm-app-platform.md:383-412` |
| 宿主方法（封闭清单） | `db.define/db.query/db.exec/tx_begin/tx_commit/tx_rollback/log/assets.read` — 无文件、无网络、无 env、无时钟配置、**无 AI** | 同上 §5.1、§7.2 |
| 计时 | 端到端 60s（含排队）> guest 30s（进宿主调用暂停）> 单条 SQL 5s | `limits/limits.go:310,380-400` |

### 8.2 每应用数据库

| 项 | 事实 | 证据 |
|---|---|---|
| 引擎 | 每应用一个 SQLite 文件 `<data_root>/apps/<app_id>/app.db`（modernc，1 写 + N 读、WAL、`busy_timeout 3s`） | `wasmapp/appdb/appdb.go:1-40`；`limits/limits.go:230-258` |
| 体积 | `max_page_count=25600` × 4KiB = **100 MB**；且 `SQLITE_LIMIT_*` 是**连接级不持久** ⇒ 每条连接重设 + 金丝雀校验 | `limits/limits.go:232-234`；`appdb.go:10-25` |
| 语句闸门 | 单语句、仅 SELECT/INSERT/UPDATE/DELETE、禁 DDL/ATTACH/PRAGMA；`ATTACH` 引擎层 `LIMIT_ATTACHED=0` 否决 | `appdb/sqlgate.go`；`limits.go:344-352` |
| 结构 | 建表只经 `db.define`：表 ≤16/应用、列 ≤16/表、类型枚举 text/int/real/bool/datetime、平台列 `_row_id` | `limits.go:319-332` |
| 隔离真相 | 应用**之间**靠文件边界；应用**之内不按用户隔离**（同应用全员共享一个库，需要按人区分由应用自己加字段） | `2026-09-17-wasm-app-platform.md:371-382` |

### 8.3 生命周期（对齐 CF Workers，迁移 0086）

```
App: active ⇄ unpublished ──freeze──▶ frozen ──▶ deleted(软删,可 restore) ──purge_after──▶ purged
Release: pending ─approve→ approved ─deploy→ 线上;  pending ─withdraw→ withdrawn;  ─reject→ rejected
Deployment: publish / review / rollback / seed / legacy（不可变、追加、永不删）
```

| 动作 | 语义 | 端点 |
|---|---|---|
| 上传/发布 | `validate` → `releases`（**同步**：编译+落行；失败不落行、不占版本号） | `POST /apps/wasm/validate`、`/:app_id/releases`（`optional_apps_on.go:34-35`） |
| 分片续传 | 8 MiB/片，≤1024 片，会话 30min，单用户 4 会话、48 MiB 磁盘配额 | `limits.go:125-148`；`optional_apps_on.go:130-134` |
| 上/下架 | 作者自主，与审核开关无关；下架=内容冻结（新版本上传/approve 一律 409 `APP_DELISTED`） | `optional_apps_on.go:36-38` |
| 回滚 | 用旧 approved 版本再部署一次（不重传不重审，追加 deployment） | `POST /:app_id/deployments` |
| 删除/恢复/真删 | 软删+快照（快照失败=删除失败）→ 保留期 → 调度器 `wasm_retirement` 每日真删（元数据留 tombstone） | `docs/planning/2026-10-22-wasm-lifecycle-redesign.md:189-215` |
| 导出 | `GET /:app_id/export`（JSON；`?database=1&assets=1` 出 zip），每次审计 | 同上 5.6 |

### 8.4 限额、鉴权、AI 与审计

| 项 | 事实 | 证据 |
|---|---|---|
| 并发 | 每应用 4（读并发，写串行）、每用户每应用 1 跑/4 排队、每用户全局 4、全局 32、每应用队列 32（429+Retry-After） | `limits.go:398-405`；`2026-09-17-wasm-app-platform.md:217-233` |
| 可调项 | 控制台「应用平台」可调 `app_running`/`app_db_readers`/`guest_budget_seconds`/`request_wall_clock_seconds`；**单应用**可覆盖四个并发轴（迁移 0087，NULL=继承全局） | `wasmapp/applimits/appoverride.go`；`mig/0087` |
| 鉴权 | 唯一入口 `POST /apps/wasm/:app_id/request` 必须 BearerAuth；**无匿名面**；作者面鉴权 = `ownedApp`（非 owner 一律 404，与"不存在"逐字节同形） | `optional_apps_on.go:78-84` |
| 访问控制 | 应用配置 `zeokeel.app.json` 的 `access ∈ login/whitelist`（`public` 已废除）；`whitelist` 名单**由应用自己判**（平台不比对、不提供员工目录、不校验账号存在性） | `2026-09-17-wasm-app-platform.md:147-160,412-425` |
| AI 能力 | **服务端没有任何 AI 能力**：`ai.chat` 宿主函数已于 2026-09-19 删除；应用内 AI = 客户端前端桥 `POST /__zeokeel/ai/chat`（本地处理、走员工自己的 LLM 链路） | `2026-09-17-wasm-app-platform.md:186-200`；`abi/abi.go:34-46` |
| 应用维度归因 | 网关从客户端会话 id 的 `app:` 前缀派生 `app_id` 落 `usage.app_id`；服务端只校验**形状 + 应用存在**，**不校验调用方与应用关系** ⇒ 该口径可被任何员工污染，**不得用于对账/计费/授权** | `llmgateway/app_attribution.go:1-45` |
| 审计 | `audit_logs`（管理动作）+ `wasm_call_events`（调用事件证据，保留 7 天）+ `wasm_app_opens(_daily)`（打开计数，日汇总长期保留）+ 作者/管理员数据面浏览单独审计 | `mig/0069-0075,0089`；`optional_apps_on.go:45-52` |

### 8.5 客户端如何打开（origin / deep link）

| 项 | 事实 | 证据 |
|---|---|---|
| 打开方式 | 独立应用窗口加载 `<渠道 app scheme>://<app_id>/<path>`；分享深链 `<scheme>://app/<app_id>[?path=]` | `2026-09-17-wasm-app-platform.md:460-470` |
| Scheme | 渠道配置 `desktop.app_origin_scheme`（官方 `zeokeel-app`）、`desktop.deep_link_scheme`（官方 `zeokeel`）；服务端读同一份做自源校验，两端不一致 ⇒ 所有非幂等请求 403 | `channel/channel.go:60-120` |
| 请求路径 | 客户端协议 handler 补 `Origin: <scheme>://<app_id>`，把请求包成信封 → `POST /api/client/v2/apps/wasm/:app_id/request`（身份由客户端注入的 Bearer 决定） | `optional_apps_on.go:78-84`；`docs/decisions/2026-09-19-wasm-client-internal-origin.md` |
| 打开计数 | 每次打开调 `POST /:app_id/open`；响应头 `X-ZeoKeel-App-Version` 是客户端内容缓存键的**唯一来源** | `optional_apps_on.go:96-98` |
| 已删除 | 应用子域/换票 `/login`、`/logout`、`/app-ticket`、应用 Cookie 会话、匿名入口——全部删除 | `router.go:110-115`；`mig/0073` |

---

## 9. API 面

| 域 | 端点前缀 | 鉴权 | 代表端点 |
|---|---|---|---|
| 认证 | `/api/client/v2/auth` | 无/Bearer | login、logout、me、usage、password、methods、`{oidc,openid}/{login,callback}` |
| 启动配置 | `/api/client/v2/config/bootstrap` | Bearer | 默认模型 + 模型清单 + 技能清单 + web 参数 + 连接器 |
| 渠道/更新 | `/api/client/v2/channel[/logo,/logo-dark,/favicon]`、`/updates/manifest`、`/updates/client/*` | 公开 | 登录前就要用的品牌与安装包 |
| 内容（员工） | `/api/client/v2/{skills/builtin,shared-skills,agent-presets,capabilities,marketplace/skills}` | Bearer | 目录/详情/归档/上传 |
| WASM（员工） | `/api/client/v2/apps/wasm/*` | Bearer | validate/releases/deployments/request/open/proof/uploads/rows/schema/export |
| 遥测 | `/api/client/v2/telemetry/*` | Bearer | skill-call、error-reporting |
| LLM 网关 | `/v1/*`（另有无前缀别名） | Bearer | chat/completions、completions、responses、embeddings、messages、models、files |
| 管理面 | `/api/server/admin/*` | Cookie+CSRF+RBAC | users/departments/balance/usage/providers/models/gateway/connectors/skills/agent-presets/wasm-apps/audit/config/report-subscriptions/portal |
| 公开面 | `/`、`/portal`、`/healthz`、`/admin/` | 无 | 门户（纯 HTML/CSS 零脚本）、健康探针、内嵌 SPA |

约定：

| 项 | 事实 | 证据 |
|---|---|---|
| 错误信封 | `{"error":{"code":"ERR_CODE","message":"...", "details"?, "hints"?}}`；统一 `WriteError` | `handler.go:335-337` |
| 错误码 | `AUTH_REQUIRED/AUTH_FAILED/FORBIDDEN/NOT_FOUND/VALIDATION/UPSTREAM/RATE_LIMITED/BALANCE_EXHAUSTED/MODEL_NOT_PRICED/PASSWORD_CHANGE_REQUIRED/APP_DELISTED/APPROVED_NOT_REJECTABLE/INTERNAL` | `server/docs/03-api-reference.md:9-24` |
| 版本策略 | 员工面**只有 v2 一个活跃大版本**，旧前缀（`/api/auth`、`/api/marketplace`、`/v2/api/*`）**直接移除、不做兼容层**；管理面无版本号；网关跟上游官方形状 | `router.go:1-14`；`server/docs/03-api-reference.md:3-6` |
| 兼容手法 | 新字段只增不改；下线字段保留"接收但忽略"（如 `quota_*`），以免砸旧客户端；字段集合用跨语言契约测试对拍 | `server/docs/03-api-reference.md:39,86` |
| 授权语义 | 未授权资源一律 **404**（与不存在同形，不泄露存在性）；RBAC 拒绝是 403 | `serverstore/distribution.go`；`rbac.go:120-126` |

---

## 10. 部署与分发

| 项 | 事实 | 证据 |
|---|---|---|
| 镜像 | 3 阶段：node 构建 webadmin → golang 交叉编译（`CGO_ENABLED=0`，ldflags 注入版本）→ alpine 运行（uid 10001、su-exec 降权、`HEALTHCHECK`、`VOLUME /data`） | `server/Dockerfile:1-180`；`server/docs/02-build-deploy.md:70-76` |
| 镜像内自带 | compose + 三份 Caddyfile + `.env.example` + `VERSION` + `CHANNEL`（客户侧一个 `docker run` 即可导出整套部署栈） | `server/Dockerfile:182-198` |
| 客户端安装包 | CI 三平台产物 → 构建上下文 `clientassets` → 镜像 `/opt/zeokeel/client/`（含 `CLIENT-RELEASE.json`）；服务端直接对外：`GET /api/client/v2/updates/manifest` 与 `GET /updates/client/<file>` | `server/Dockerfile:205-226`；`clientrelease/clientrelease.go:1-20,76-120` |
| 为什么随镜像 | 客户端从**它登录的这台服务端**取包 → 员工机不需要外网，且"客户端版本与服务端版本"天然配套（结构上不可能错配） | `clientrelease/clientrelease.go:1-9` |
| 门户下载页 | `/`、`/portal` 纯 HTML+CSS、零脚本（CSP `default-src 'none'`）；首屏即三平台下载卡片，按渠道 `desktop.platforms` 收窄；管理入口只在页脚 | `portal/portal.go:1-30`；`channel/channel.go:76-86` |
| 更新服务器 | `https://release.zeokeel.com/<channel>/latest.json`；服务端 `updatecheck` 只认 **channel_id 与自身渠道相等**的清单（不一致 ⇒ 检查不可用，绝不跨渠道升级）；结果经 `GET /admin/server-info` 的 `update_check` 暴露 | `updatecheck/updatecheck.go:1-50`；`server/docs/03-api-reference.md:72` |
| 发布通道 | `push tag v*` → 四路（server/linux/win/mac）全绿 → 按渠道构建镜像 → `docker save|zip` → GitHub Release（仅 beta/official）+ 上传 `<channel>/releases/<v>/`（同目录 SHA256SUMS，保留最近 3 版）；**不经任何镜像仓库**（GHCR 已下线） | `server/docs/02-build-deploy.md:78-100` |
| 许可证 | **无任何 license/entitlement 校验**（全仓无相关代码）——商业边界靠交付形态：镜像只从渠道更新服务器取、渠道内自带客户端包与渠道内容 | `grep -r license server/internal`（无命中） |
| 渠道如何影响服务端 | ①镜像内客户端包与渠道内容由构建期注入；②更新清单归属（channel_id 必须一致）；③深链 scheme / 应用 origin scheme；④门户下载平台列表与品牌文案；⑤`features_variant` 决定编进哪些模块（skills/apps/connectors） | `channel/channel.go:1-30,60-120`；`server/Dockerfile:196-203`；`docs/decisions/2026-10-14-server-build-tags.md:28-40` |
| 落库配置 | 运行期旋钮优先控制台改（`GET/PUT /admin/config`），`restart` 类键需重建容器；env 覆盖会在 `warnings` 里点名 | `server/docs/03-api-reference.md:73-74` |

---

## 11. 对 Rust + zeokeel-cordis 双端的设计经验

### 5 条最值得抄

| # | 经验 | 为什么值钱 | 参考实现 |
|---|---|---|---|
| 1 | **路由唯一真源 + 权限点同处申报**：一张 `Register()` 里同时写路径/方法/中间件/`权限点`，业务模块只交 handler | 路由可枚举 ⇒ 可写"全量 API 扫描测试"（未授权端点、必须 JSON、大 body 白名单都能机械断言）；Rust 侧可用单一 `Router` 构建函数 + `axum` 中间件层，把权限常量做成编译期枚举 | `router.go:88-93,314-444`；`router.go` 里的 `largeBodyRoutes` 白名单 |
| 2 | **一套内容模型覆盖多产品形态**：`apps(kind, app_id, channel, owner, enabled, official)` + `app_releases(不可变版本, archive BYTEA)` + `app_grants(grantee_type, grantee)`，技能/Agent/WASM 只是 `kind` 不同 | 审批、归属、授权、下架、配额、审计只实现一次；三条上传路径共用一个 `Publish` 内核、一个 `Writable()` 判据 | `mig/0053`；`serverstore/distribution.go`；`appstore/publish.go` |
| 3 | **网关四段式：准入 → 转发 → 计量 → 结算**，且"钱"的判据全部在转发前、结算与 usage 同事务 | 白烧上游是这类系统最大的静默成本；把"估算口径"与"结算兜底"做成**同一函数**，再叠加"学到的下限"和"未定价默认拒绝"，把可无限重复的漏洞收成"每次充值最多漏一次" | `llmgateway/balance_gate.go:1-45`；`balance.go:236-315`；`handler.go:113-118` |
| 4 | **数值单一真源 + 生成物 + 变异测试**：所有限额集中在 `limits.go`，同时生成 `limits.json`／技能文档／错误文案；导入面白名单由"参考实现真编译 dump"生成，手写即测试红；每条新连接必须重设 SQLite 限额并有金丝雀 | Rust 侧同理：`const` 一处定义 → `build.rs` 生成 TS/JSON/Skill 文档；把"限制被改回无上限"做成会红的变异用例 | `limits/limits.go`（全文件）；`docs/planning/2026-09-17-wasm-app-platform.md:330-343` |
| 5 | **交付面自包含**：镜像里带 compose/Caddyfile/客户端安装包/内置技能/渠道内容，客户端从登录的这台服务端取包；渠道在**构建期**注入而不是运行期改品牌 | 内网部署零外网依赖、版本天然配套、品牌不可被线上篡改（每次改都可审计、可追溯） | `server/Dockerfile:182-230`；`clientrelease/clientrelease.go:1-20`；`channel/channel.go:1-15` |

补充（值得借鉴但成本高）：**编译期模块裁剪**（`skills/apps/connectors` build tag + 变体矩阵门禁）让同一仓库产出"全功能/仅应用/无内容"多个变体，裁剪是编译期事实而不是运行时开关（`router/optional_*.go`；`docs/decisions/2026-10-14-server-build-tags.md`）。

### 3 个必须避开的设计

| # | 反模式 | 现场证据 | 我们应该怎么做 |
|---|---|---|---|
| 1 | **迁移"可改写"且无修复通道**：`schema_migrations` 起初只有 `(version, applied_at)`，已发布迁移被就地改写不会被发现；出错后**没有任何重放/修复入口**，部分库永久不可恢复（`0054/0055` 的修复对已升级库不可达） | `server/docs/06-database.md`「已知边界」+ 事故清单（`0004` 导致**崩溃循环且重试永不自愈**） | 第一天就做：迁移文件**不可变**（内容哈希入库并在启动对账，不一致 fail-loud）+ 只允许**前向**修复迁移 + 版本表带 checksum/时间/执行者；Rust 侧用 `sqlx::migrate!` 也要自己加内容指纹 |
| 2 | **多套并行额度**：token 月配额 + 金额月配额 + 部门预算 + 账户余额四套闸门同时存在，互相打架（"充了钱仍被软配额拦住"），下线后**列与请求字段长期保留**，客户端/管理端/文档各自保留一套说法 | `server/docs/01-architecture.md:38-52`（"别再把它们当现役闸门"）；`balance.go` 与 `users.quota_*` 并存 | 只保留**一本账**（余额/额度二选一）+ 明确弃用流程（先停止写入 → 停止读取 → 新迁移删列），不要让"兼容字段"活过两个版本 |
| 3 | **生命周期只有隐式指针、没有执行者**：WASM 应用"生效版本 = 最新 approved"的隐式推导 ⇒ 无回滚、无部署历史；冻结/退役/删除在 UI、审计链、限额表里四处**承诺**却无人执行，最后必须整体重设计（引入 deployment 表 + 退役调度器 + 软删/恢复/真删三段） | `docs/planning/2026-10-22-wasm-lifecycle-redesign.md:29-50`（"四处假承诺"） | 内容/应用从第一天就有 `deployment`（不可变动作 + actor + kind）与状态机；"删除/冻结/保留期"必须有**真实后台作业**与状态出口，否则不要写进 UI 与文档 |

> 附带一条同族教训：**不要把 AI 能力做进沙箱/服务端宿主**。`ai.chat` 曾作为 WASM 宿主函数存在（含独立 30s 预算、服务端模型裁决、按使用者计费），2026-09-19 整体删除，改为客户端前端桥 + 员工自己的 LLM 链路，并留下"老应用导入即 `IMPORT_NOT_ALLOWED`"的迁移成本（`docs/planning/2026-09-17-wasm-app-platform.md:186-200`；`abi/abi.go:34-46`）。

---

## 附：文档漂移（读文档时注意）

| 文档说法 | 代码事实 | 证据 |
|---|---|---|
| WASM guest 执行预算 10s | **30s**（2026-10-01 因"PDF 打水印"类应用放宽；控制台可覆盖） | `docs/planning/2026-09-17-wasm-app-platform.md:217` vs `limits/limits.go:386-400` |
| 技能归档 ≤16MB | 原始归档 16 MiB、请求体 24 MiB（base64 膨胀），两者不同 | `server/docs/03-api-reference.md:187` vs `archiveutil/archive.go:39`、`sharedskills/routes.go:40` |
| 网关限流默认 60/min | 代码缺省 **0（不限）**；老库已存 60 不会自动改 | `server/docs/04-auth.md:74-84` |
| "部门预算约束网关" | 已下线，`groups.budget_money` 只读不写 | `server/docs/06-database.md` groups 一节开头 vs `server/docs/01-architecture.md:38-52` |
