# 04 · 契约与 API：客户端 ↔ 服务端

> 一句话：**客户端是本地 agent，服务端是认证/用户/钱包/网关/内容分发/WASM/审计/管理的唯一权威。**
> 双端都是 cordis 包化、构建期组合；进程间载体 = HTTP JSON + SSE（网关流式）+ WS（服务端事件）+ WASM 信封。
> 本文只定契约面与规则，不写实现。管理端 API、服务端领域模型与部署见 `06/07/08`；未定项集中 §11（索引 [`15` §C](15-open-questions.md#c-待定清单)），未定处标「默认值」并给收敛点。个人版纯客户端、无服务端（15 §A D9），不适用本文 API 面。

## 1. 契约分层

**先划边界（已拍板）**：客户端负责本地 agent、工具/技能执行、MCP 连接、审批、本地存储与 UI，不碰用户/角色裁决、钱包、token 签发与服务端策略；服务端只做认证、用户、钱包、AI 网关、内容分发、WASM、审计与管理，不执行 agent/技能、不代理 MCP 流量、不下发上游 key。传输只有 HTTP JSON + SSE（网关流式）+ WS（服务端事件）+ WASM 信封，本期不用 gRPC/Protobuf。

**五层契约（各有唯一真源，只能向下依赖）**

| 层 | 唯一真源 | 产物/载体 | 兼容与失效方式 |
| --- | --- | --- | --- |
| Rust trait 契约（同进程、包间） | `zk-contract-*` crate，一 major 一 crate | trait + DTO + conformance 套件；双端每个域包只依赖契约 | 跨 major = 新 crate；不匹配在编译期失败（ADR-002/016） |
| HTTP API 契约（客户端↔服务端） | 服务端 manifest `[[routes]]` 静态声明 + `zk-contract-api` DTO；运行期 `Register()` 校验 | `shared/contracts/http-routes.json`、`openapi.json`；客户端 `server-connector` 包 | v1 内只增不改；破坏性变更开 v2（§4） |
| 事件契约（WS） | `zk-contract-events` DTO + topic 注册表 | `events.json`；客户端按 topic 订阅 | 只增字段/只增 type；未知 type 忽略并记日志 |
| UI 契约 | `zeokeel/ui-shell-kit` 包（UI 包，非契约 crate）的 SlotDefs/props 类型 + 各包 `[[ui.contributions]]` | `ui-contributions.json`；shell 只认 componentId/slotId | 构建期闭包校验失败即构建失败（ADR-017） |
| 数据契约（DTO/serde） | `zk-contract-api` 与领域契约 crate 里的 serde 类型 | schemars → JSON Schema → `scripts/schema-ts.py` → TS interfaces + `SCHEMAS`；前端 Ajv；serde 版本字段 | 只增字段 + `#[serde(default)]`；枚举增值客户端降级 Unknown |

三条硬规则：

1. UI **不得直连**服务端：UI → 客户端 `ui.transport` → 客户端包；服务端 HTTP 只允许 `server-connector` 一个包发出（包名以 M1 包清单为准，见 [`15` §C](15-open-questions.md#c-待定清单)；便于 mock/录制回放、离线队列与统一 401 处理）。唯一例外是 WASM 应用窗口，经协议 handler（§9）。
2. 双端都是包：服务端 auth/users/wallet/gateway/skills/connectors/apps/audit/admin 各为包，只经契约与路由表耦合；承载 HTTP 的包可替换（对应 `platform.*` 角色）。
3. 一份 DTO 双侧单实例：客户端与服务端在同一 workspace 依赖**同一个**契约 crate 实例；若将来拆仓，契约必须发布并按版本锁定（与 ADR-016 同一理由）。`zk-contract-api`/`zk-contract-events` 属 M1 新增契约，须在根 `zk.toml` 的 `[contracts]` 登记（[包模型](02-package-model.md) §5）。

## 1.5 弱管控边界（v3.2 第一原则）

企业版**不是强管控终端**：服务端只管身份、钱、商店内容、审计、版本；客户端能力完整，工具默认全开、
网络不限制、不做企业审批。契约层只保留三类硬限制与通用工程约束：

| # | 原则 | 契约层落地 |
| --- | --- | --- |
| C1 | 服务端不控制工具/网络/审批 | 不下发工具白名单、网络策略、审批策略；客户端本地审批由用户决定；API 中不存在 `POLICY_DENIED` 这类"服务端禁止操作"错误 |
| C2 | 能力完整下发 | bootstrap **只下发**模型目录、商店目录（技能/连接器/应用）、更新清单、渠道/品牌信息；不下发"能力开关" |
| C3 | 三类硬限制 | ① 企业版 AI 只能走服务端网关（客户端不持上游 key、不填 base_url）；② 企业内容只能从企业商店安装；③ 禁用用户令牌立即失效 |
| C4 | 管理写操作四件套 | 权限点 + 幂等 + 审计 + 限流；高危操作 step-up 二次确认（仅管理台） |
| C5 | 错误码穷举 | 封闭业务码：`WALLET_EXHAUSTED`、`MODEL_NOT_PRICED`、`MODEL_NOT_FOUND`、`RATE_LIMITED`、`CONTENT_NOT_AVAILABLE`、`CLIENT_TOO_OLD` 等；未列出错误只回 `INTERNAL + request_id` |
| C6 | 一致性/可预期 | cursor 分页；写操作返回 `operation_id`；`min_client_version` + capabilities 协商；未知字段/枚举/事件 type 忽略；客户端上报只作运营指标，不作合规证据 |

路由表全量扫描 + 负向用例进 CI（§5、§10）。**注意**：C1 意味着安全责任在客户端与客户网络边界，
服务端不为员工行为兜底；文档与 UI 不得暗示"服务端可以限制员工操作"。

## 2. 客户端 API 列表（按域）

约定：下表路径省略公共前缀 `/api/client/v1`；跨 major 稳定面（`/api/meta/*`、`/api/channel`、`/api/updates/*`）与网关（`/v1/*`）写全路径。默认需 Bearer JWT，`public` 显式标注；所有请求带 `X-ZeoKeel-Client: <semver>;<channel>;<platform>;<device_id>` 与 `X-Request-Id`；列表一律 cursor 分页；错误见 §7。**路由表是路径/方法/权限的唯一真源**（§5），本表只列代表性端点，不逐字段穷举。

### 2.1 auth — 登录与会话

| 方法与路径 | 用途 | 鉴权/权限点 | 请求/响应要点 | 幂等 | 主要错误 |
| --- | --- | --- | --- | --- | --- |
| `POST /auth/login` | 账号密码登录，建立会话（**不传服务端地址**：非空时来自渠道锁定；空时来自首配页本地保存，设置可改） | public | req: username/password、device{id,name,platform,channel}、client_version；resp: access_jwt、refresh_token、expires_in、user+permissions 摘要、password_must_change | 否（同设备重复登录 = 新会话） | AUTH_FAILED、ACCOUNT_DISABLED、RATE_LIMITED、CLIENT_TOO_OLD |
| `POST /auth/refresh` | 轮换 access+refresh | refresh_token | req: refresh_token、device_id；resp: 新 token 对 | 否；旧 refresh 重放 ⇒ reuse 检测 | AUTH_EXPIRED、REFRESH_REUSED、DEVICE_MISMATCH |
| `POST /auth/logout` | 吊销当前设备会话 | Bearer + refresh | 204 | 是（重复仍 204） | AUTH_REQUIRED |
| `GET /auth/me` | 当前用户/角色/权限点/设备 | Bearer `auth:self` | resp: user、role、permissions[]、devices[] | 是（安全方法） | AUTH_EXPIRED |
| `GET /auth/methods` | 登录前认证方式与 IdP | public | resp: mode(local/oidc/ldap)、providers[]、min_client_version | 是 | — |
| `POST /auth/oidc/exchange` | 浏览器 SSO 回调深链换会话 | public（一次性 code+state） | req: code/state/device/client_version；resp 同 login | 否（code 一次性） | OIDC_FAILED、STATE_MISMATCH |
| `GET /wallet` · `GET /wallet/ledger` | 钱包余额/本月用量与流水（只读） | Bearer `wallet:self` | resp: balance_display、activated、month_granted、month_used、today；ledger 为 cursor 分页流水（含 usage_id） | 是 | — |

钱包充值/调整/流水在管理端（另文）；网关扣费在服务端内部完成（§2.3）。

> **登录地址**：来自构建期 `channel.json.defaults.server_url`，不通过登录请求传递。非空 → 客户端锁定、登录页只显示账号/密码；
> 为空（官方企业版默认）→ 首配页显示地址输入，本地持久化（设置中可改，需重新登录）。

### 2.2 bootstrap/config — 配置下发

| 方法与路径 | 用途 | 鉴权/权限点 | 请求/响应要点 | 幂等 | 主要错误 |
| --- | --- | --- | --- | --- | --- |
| `GET /api/meta/capabilities` | 版本与能力协商（登录前可用） | public | resp: api_version、server_version、min_client_version、features[]、auth_methods[]、app_origin_scheme、event_topics[]；ETag | 是；304 | — |
| `GET /config/bootstrap` | 登录后全量配置 | Bearer `config:read` | resp: schema_version、identity、gateway（模型目录/默认模型）、skills、connectors、apps、updates、brand、config_etag；**不含工具/审批/网络策略**；`If-None-Match`→304 | 是 | CLIENT_TOO_OLD、CONFIG_UNAVAILABLE |
| `GET /api/channel` | 品牌与渠道（登录前也要） | public | resp: channel_id、brand{...}、assets 基址；ETag | 是 | — |

`/api/meta/*`、`/api/channel`、`/api/updates/*` 是**跨 major 稳定面**（只增不改），老客户端才能在升级前完成发现与自更新；`/api/client/v1/*` 是业务面。网关 `/v1/*` 跟 OpenAI 形状，版本由上游协议决定。

### 2.3 gateway — AI 网关

| 方法与路径 | 用途 | 鉴权/权限点 | 请求/响应要点 | 幂等 | 主要错误 |
| --- | --- | --- | --- | --- | --- |
| `POST /v1/responses` | **唯一网关协议**：OpenAI Responses API | Bearer `gateway:chat` | input/instructions/tools；返回 response 对象或 SSE 流；工具调用子树透传 | 否；仅在首字节前可换 provider | WALLET_EXHAUSTED、MODEL_NOT_PRICED、RATE_LIMITED、MODEL_NOT_FOUND、UPSTREAM |
| `GET /v1/models` | 当前用户可用模型/能力/展示价 | Bearer `gateway:chat` | resp: data[{id,capabilities,context,pricing_display}] | 是 | — |

规则：`base_url` 来自 bootstrap，客户端**不持有上游 key**；上游选择、故障转移、计量与扣费全在服务端（准入在转发前、usage 与账本同事务）；错误不回显上游细节；流式中断以 SSE error 帧显式结束。

### 2.4 skills — DSH 技能分发

| 方法与路径 | 用途 | 鉴权/权限点 | 请求/响应要点 | 幂等 | 主要错误 |
| --- | --- | --- | --- | --- | --- |
| `GET /skills` | 可见技能目录（approved ∧ 上架 ∧ 授权） | `skills:read` | resp: items[{name,version,title,category,sha256,min_client_version,enforced}] | 是 | — |
| `GET /skills/{name}/{version}/archive` | 下载归档（zip/tar.gz） | `skills:read` | octet-stream；响应头 `X-ZeoKeel-Skill-Sha256`（**归档哈希**；整树哈希客户端计算，ADR-061） | 是 | SKILL_NOT_FOUND、CHECKSUM_MISMATCH |
| `POST /skills` · `DELETE /skills/{name}/{version}` | 上传新版本（员工待审；管理员代传即通过）／撤回（仅 pending） | `skills:publish` | multipart: archive + manifest；resp: status(pending/approved)、version | 否；同 name+version 不可覆盖（409） | CONFLICT、VALIDATION、PENDING_LIMIT、QUOTA_EXCEEDED |

规则：服务端**只存/审/分发**，技能在客户端执行（DSH 格式：`SKILL.md` + frontmatter + 随包脚本）；服务端只读归档元数据并计算归档 sha256（ADR-061），整树 sha256 由客户端解压后计算/重算，不一致拒绝执行；版本号永久占位、同版本不可覆盖；强制更新比较版本 + 归档 sha256（见 §8/§11）。

### 2.5 connectors — 企业 MCP 下发

| 方法与路径 | 用途 | 鉴权/权限点 | 请求/响应要点 | 幂等 | 主要错误 |
| --- | --- | --- | --- | --- | --- |
| `GET /connectors` | 企业 MCP 定义目录 | `connectors:use` | resp: items[{id,name,transport,auth_mode,scopes,enabled}]；**不含密钥** | 是；ETag | — |
| `POST /connectors/{id}/{enable,disable}` | 员工启停（记偏好，不影响他人） | `connectors:use` | 幂等设置；resp 当前 enabled | 是 | CONNECTOR_UNAVAILABLE |

规则：**连接器定义/公共客户端元数据**由服务端存储、只下发不含 secret 的配置；**OAuth 授权码流程在客户端完成**（本机回调/自定义 scheme），access/refresh token **客户端本地保存**（D53），服务端不代理 MCP 流量；定义唯一真源在服务端、客户端只读；**企业版允许 stdio**（D33，管理员配置命令/参数/环境变量），个人版本地自配（D19）。

### 2.6 apps — WASM 应用

| 方法与路径 | 用途 | 鉴权/权限点 | 请求/响应要点 | 幂等 | 主要错误 |
| --- | --- | --- | --- | --- | --- |
| `GET /apps` | 可见应用目录 | `apps:use` | resp: items[{app_id,name,description,icon,version,access,ui}]；ETag | 是 | — |
| `POST /apps` | 员工上传应用（pending） | `apps:publish` | multipart `manifest`(JSON) + `wasm` + `ui` zip；owner=当前用户；resp: app_id/status pending | 否 | VALIDATION、CONFLICT、APP_BAD_WASM |
| `GET /apps/{app_id}/assets/{path}` | 应用静态 UI 资源 | `apps:use` | 只读 release `ui/` 清单内文件；ETag；无 token 泄露 | 是 | APP_NOT_FOUND、APP_DELISTED、NOT_FOUND |
| `POST /apps/{app_id}/open` | 打开计数 + 版本票据 | `apps:use` | resp 头 `X-ZeoKeel-App-Version` + `X-ZeoKeel-App-Ui`（缓存键唯一来源）；不返回可执行 guest | 否（计数） | APP_NOT_FOUND、APP_DELISTED |
| `POST /apps/{app_id}/request` | 请求信封转发（应用全部数据面） | `apps:use` | req/resp 见 §9 | 由应用语义决定；GET 安全、写不幂等 | APP_DELISTED、RATE_LIMITED、APP_UNAVAILABLE |
| `GET/POST /api/admin/apps` | 管理列表/创建 | `apps:admin` | 列表含 pending/approved/version；创建 admin-only | 是/否 | VALIDATION |
| `POST /api/admin/apps/{id}/versions` | 管理上传/释放 | `apps:admin` | multipart manifest+wasm+ui；同版本 409 | 否 | APP_BAD_WASM、CONFLICT |
| `POST /api/admin/apps/{id}/{approve,reject,delist,rollback,delete,purge}` | 审核/上下架/回滚/退役 | `apps:admin` | super_admin + CSRF；audit | 否 | APP_NOT_FOUND、CONFLICT |
| `PUT /api/admin/apps/{id}/grants` | 可见/可用 grants | `apps:admin` | all/user + can_open | 否 | VALIDATION |
| `GET /api/admin/apps/{id}/diagnostics` | 运行诊断 | `apps:admin` | calls/logs/limits，脱敏 | 是 | APP_NOT_FOUND |

应用发布/审核/部署/导出在管理端与发布面（另文）；客户端**不执行应用、不持有应用 DB**。

### 2.7 updates — 客户端更新

| 方法与路径 | 用途 | 鉴权/权限点 | 请求/响应要点 | 幂等 | 主要错误 |
| --- | --- | --- | --- | --- | --- |
| `GET /api/updates/manifest` | 企业版客户端更新清单（仅服务端提供） | public | resp: schema、channel_id、edition、**server_version、client_version、min_supported、forced、rollout?、deadline?、previous?**、assets{url,sha256,size}、signature；ETag | 是 | channel_id/edition 不匹配或 client_version≠服务端 `CLIENT-RELEASE.json` → 拒绝整份清单 |
| `GET /api/updates/client/{artifact}` | 安装包下载 | public | octet-stream，支持 Range/断点续传；sha256 独立校验 | 是 | NOT_FOUND |

规则：企业版客户端**只从登录的这台服务端**取更新（二进制里没有指向 GitHub/公共更新面的路径），`channel_id`/`edition` 必须匹配，`client_version` 必须等于服务端镜像内 `CLIENT-RELEASE.json` 的版本（1:1）；低于 `min_supported` 时业务面 426，稳定面始终可用于升级。**个人版不走此接口**，直接查 GitHub Releases（`latest.json` + `SHA256SUMS`）。

### 2.8 audit/telemetry — 审计与遥测

| 方法与路径 | 用途 | 鉴权/权限点 | 请求/响应要点 | 幂等 | 主要错误 |
| --- | --- | --- | --- | --- | --- |
| `POST /telemetry/events` | 批量上报（技能调用/错误/崩溃/健康） | `telemetry:write` | req: schema、batch_id、events[{event_id,type,ts,attrs}]（≤100/批；崩溃用 crash_id + 包清单，**不含会话内容/密钥**） | 是：event_id/crash_id 去重（窗口 7 天） | VALIDATION、RATE_LIMITED、PAYLOAD_TOO_LARGE |
| `GET /audit/me` | 我的安全与审计事件 | `audit:self` | resp: 分页事件（登录/设备/钱包/技能安装/管理动作影响） | 是 | — |

规则：客户端上报**不作为权威审计**；权威审计在服务端（管理端查询另文）；离线时本地有界队列、best-effort 补传；默认关闭、opt-in、仅匿名计数与元数据（Q-S14）。

## 3. 认证与会话契约

| 项 | 规则 |
| --- | --- |
| 认证主源 | 服务端只启用**一种主源**：local / OIDC / LDAP（三选一）＋一个本地应急超管账号；应急账号仅 break-glass，不参与日常登录，登录必审计并告警 |
| access JWT | 短效 15 min；签名 EdDSA，`kid` 轮换、算法白名单；claims: iss/aud/sub/uid/role/epoch/device_id/channel/jti/iat/exp；服务端用 `users.token_epoch` 实现全局失效（缓存窗口默认值 5 s，M2 spec 确认，15 §C；管理面写操作强制查库） |
| refresh token | 不透明随机串（非 JWT），服务端只存哈希；默认值 30 天绝对寿命（M2 spec 确认，15 §C）；每次刷新轮换；重放检测：旧 token 被用 ⇒ 吊销该设备整个会话族。refresh 绑定 `device_id` + `channel`，access 携带两者，跨设备使用一律拒绝 |
| 刷新时机 | 过期前 60 s 主动刷新；单飞（single-flight）避免并发刷新；刷新失败 = 会话结束，清本地会话回登录页 |
| 401 处理 | 收到 `AUTH_EXPIRED` ⇒ 刷新一次 ⇒ 重放原请求一次；仍 401 ⇒ 登出；不做无限重试；403 不刷新 |
| 吊销 | logout、改密、降权、禁用、管理员强制下线、设备移除；refresh 立即失效；access **每请求校验 `token_epoch`**（≤5s 缓存，写操作强制查库），禁用/降权后下一次请求即 401；WS 推 `session.revoked`（§8） |
| 设备标识 | 安装时生成 UUID，持久化于本地数据根；登录上报 name/platform/channel；管理端可见并可单独吊销；重装 = 新设备 |
| 离线 | 本地 agent 可用；服务端依赖功能（AI/技能安装/连接器/应用/更新）以类型化错误降级；缓存 bootstrap（ETag/到期）与已装技能；禁止离线续期 |
| 管理端会话边界 | 独立 cookie 会话（HttpOnly/SameSite/CSRF；TOTP MFA 可选，D53），12h 上限 + 60min 空闲；客户端 JWT 不得用于管理面，管理 cookie 不得用于客户端面；细节见服务端/管理端文档（另文） |

## 4. 版本策略

| 项 | 规则 |
| --- | --- |
| 路径语义 | `/api/client/v1` = 客户端业务面唯一活跃大版本（路径只带 major）；v1 内只增字段/只增端点/只增枚举值，不改语义 |
| 破坏性变更 | 开 `/api/client/v2`；服务端可并行支持 v1/v2 一个过渡期，但**同时只有一个"主"版本**；不给 v1 加兼容 shim（参考实现教训：[harness-server.md](../research/harness-server.md) §11：旧前缀直接移除会砸旧客户端，所以必须先有稳定面 + 升级门禁） |
| 稳定面 | `/api/meta/*`、`/api/channel`、`/api/updates/*` 跨 major 不变、只增不改；老客户端靠它发现服务端并自更新 |
| 能力协商 | `GET /api/meta/capabilities` 返回 `api_version`、`server_version`、`min_client_version`、`features[]`、`event_topics[]`；客户端缓存并随登录刷新；未知 feature 一律忽略，不得假设存在 |
| 客户端标识 | 所有请求带 `X-ZeoKeel-Client`（版本/渠道/平台/设备）；服务端记录用于灰度与排障，不用于授权 |
| 最小版本 | 服务端按渠道配置 `min_client_version`（硬门槛；按**服务端构建渠道**读取 `ZEOKEEL_MIN_CLIENT_VERSION_<CHANNEL>`，全局 `ZEOKEEL_MIN_CLIENT_VERSION` 兜底；客户端头里的 channel 仅作排障，不参与门槛选择）与 `recommended`；低于硬门槛：业务面返回 `426 CLIENT_TOO_OLD`（稳定面、登录/刷新除外），客户端展示强制升级页 |
| 灰度/强制升级 | 客户端渠道（official/beta/oem）+ 服务端清单 `rollout`/`forced`/`deadline`；按渠道/用户比例灰度（D53，管理台配置），到期限后业务面 426；本地 agent 离线能力在升级前保持可用（宽限默认值：允许仅本地 agent，M3 spec 确认，15 §C）；同一用户不得在灰度中反复横跳 |

## 5. 权限模型与路由真源

**角色**：`super_admin`（超管，可登录管理端）、`auditor`（审计员，只读）、`employee`（普通员工，仅客户端）。权限点形如 `<域>:<动作>[:<范围>]`；员工默认持有 `auth:self`、`config:read`、`gateway:chat`、`skills:read`、`skills:publish`、`connectors:use`、`apps:use`、`telemetry:write`、`audit:self`、`wallet:self`。

| 规则 | 内容 |
| --- | --- |
| 默认拒绝 | 未在路由表声明的权限 = 不可访问；未授权资源一律 **404**（与不存在同形），RBAC 拒绝才 403 |
| 唯一真源 | manifest `[[routes]]` 静态申报方法/路径/api_version/鉴权/权限点/body 上限/幂等/审计/限流/step-up；`zk api` 生成 catalog；运行期 `Register()` 只做一致性校验（见 `24` §4）。该口径来自参考实现"路由唯一真源 + 权限点同处申报"的教训（[harness-server.md](../research/harness-server.md) §11） |
| 生成物 | 该表生成 `http-routes.json` + `permissions.json` + Rust 权限常量 + TS 权限联合类型；客户端 UI 用 `/auth/me` 的权限点做**展示层**门控（不是安全边界） |
| 门禁 | 路由缺权限点/鉴权模式/body 上限/幂等与审计策略/限流类 ⇒ 启动失败；未在生成物中的路由 ⇒ CI 失败；管理写操作无审计标记 ⇒ CI 失败；全量扫描测试枚举 public 端点并断言白名单 |
| 角色 | `super_admin` / `auditor`（只读）/ `employee`；**无部门/用户组**；内容可见性：**全员 / 指定用户** |

## 6. 配置下发与覆盖规则

登录后 `GET /config/bootstrap` 原子下发；客户端以 `config_etag` 做条件请求，失败保留 last-known-good，不进入半初始化状态。

| 配置组 | 内容 | 真源 | 本地可否覆盖 |
| --- | --- | --- | --- |
| 身份/权限 | user、role、permissions | 服务端 | 否 |
| 网关 | base_url、协议、模型目录、默认模型、上下文/价格展示、限额 | 服务端 | **企业版不允许**：provider/base_url/模型目录只读，禁止 BYOK |
| 技能 | 源地址、目录、审批状态、强制更新标记 | 服务端 | 否（本地只安装/缓存） |
| 连接器 | 企业 MCP 定义、授权范围 | 服务端 | 否（本地存 OAuth 令牌与个人凭据） |
| 应用 | 目录、origin scheme、限额 | 服务端 | 否（本地仅缓存静态资源） |
| 更新 | 清单地址、渠道、最小版本 | 服务端 | 否 |
| 品牌/渠道 | 名称、logo、主题、深链与应用 scheme | 随包 `channel.json` | 否；运行期只读，服务端只校验同渠道资产 |
| 用户偏好 | 主题、语言、布局、审批偏好 | 本地 | 是 |
| 本地偏好 | 审批偏好、工具偏好、网络设置 | 本地用户 | 是（服务端不下发这些策略） |

优先级：**服务端强制字段（身份/网关/商店/更新/渠道）> 本地用户偏好 > 包默认（profile）**。服务端字段本地覆盖一律忽略；本地覆盖不得改变鉴权、网关地址、内容源与更新源。

## 7. 错误模型

统一信封：`error{code, message, retryable, request_id, details?, retry_after_ms?}`（426 强制升级使用嵌套 `error{code,message,retryable,request_id,current,minimum,portal_url}` 并回显 `X-Request-Id`；其余 handler 逐步迁移，客户端兼容 flat/nested）。message 不含上游细节、密钥、内部路径；`request_id` 全链路透传；同一 code 的 HTTP 状态唯一、不允许一处 400 一处 422，客户端按 code 分支而不是只看状态码。

| HTTP | 代表错误码 | 可重试 | 客户端动作 |
| --- | --- | --- | --- |
| 400 | VALIDATION、PAYLOAD_TOO_LARGE | 否 | 修复请求；上报 |
| 401 | AUTH_REQUIRED、AUTH_EXPIRED、REFRESH_REUSED | 否（刷新除外） | 刷新一次；失败登出 |
| 403 | FORBIDDEN、ACCOUNT_DISABLED、ROLE_REQUIRED | 否 | 提示；不重试 |
| 404 | NOT_FOUND（含未授权资源） | 否 | 按不存在处理；不泄露存在性 |
| 409 | CONFLICT、IDEMPOTENCY_CONFLICT、APP_DELISTED | 否 | 刷新状态后人工决策 |
| 426 | CLIENT_TOO_OLD | 否 | 强制升级流程 |
| 429 | RATE_LIMITED、CONCURRENCY_LIMIT | 是（按 Retry-After） | 指数退避 + 抖动；有上限、可取消 |
| 429 | WALLET_EXHAUSTED、MODEL_NOT_PRICED | 否（需状态变更） | 充值/等管理员定价；不自动重试 |
| 500/502/503/504 | INTERNAL、UPSTREAM、UNAVAILABLE、TIMEOUT | 是（有限次） | 仅幂等请求重试；流式仅首字节前重试 |

补充规则：`retry_after_ms` 优先于本地退避；`WALLET_EXHAUSTED`/`MODEL_NOT_PRICED` 是**业务闸门**不是故障，不触发熔断；同一请求重试复用 `X-Request-Id`，写操作无幂等键不自动重试；客户端按 code 做 i18n，未知 code 走通用提示并上报。

## 8. 事件协议（WS）

| 项 | 规则 |
| --- | --- |
| 建连 | `POST /api/client/v1/events/ticket`（Bearer）→ 一次性 ticket（默认值 30 s，M2 spec 确认，15 §C）→ `wss://<server>/api/client/v1/events?ticket=…`；ticket 单次使用，避免把 JWT 放 URL/日志 |
| 信封 | `{v, event_id, type, topic, seq, ts, payload, schema_version}`；`seq` 按 topic 单调递增，与客户端本地 IPC 的 stream seq 无关（[05-client-runtime.md](05-client-runtime.md) §6）；payload 最小化，业务数据一律回源 HTTP 拉取 |
| 订阅 | 客户端发 `subscribe{topic, from_seq}` / `unsubscribe` / `ping`；服务端按 topic 维护环形缓冲（默认值 ≥1024 条或 ≥5 min，M2 spec 确认，15 §C） |
| 重放 | 断线后带 `from_seq` 续订；缓冲已过期 ⇒ 服务端回 `resync_required{topic, latest_seq}`，客户端整量重取（bootstrap/目录）后从 latest 继续 |
| 投递语义 | 至多一次推送 + 至少一次效果：客户端按 `event_id` 去重；事件只是"通知/触发"，不是状态真源（`session.revoked` 等短指令除外） |
| 恢复 | 指数退避 + 抖动重连；刷新失败或 ticket 被拒 ⇒ 登出；心跳 25 s，两个周期无 pong 即重连 |
| 服务端主动事件 | `skill.updated`/`skill.revoked`、`connector.updated`、`app.listed`/`app.delisted`/`app.version`、`config.updated`、`model_catalog.updated`、`balance.updated`/`balance.low`、`update.required`、`session.revoked` |
| 客户端动作 | 收到事件只做：失效缓存、重取对应域、提示或强制动作（升级/重新授权/登出）；不得把事件 payload 当权威数据渲染 |

## 9. WASM 应用协议

| 项 | 规则 |
| --- | --- |
| origin | 每应用独立 origin `<app_origin_scheme>://<app_id>/<path>`；scheme 由渠道配置（跨渠道唯一、≠深链 scheme）；窗口创建前注册为 privileged；每应用独立窗口 + 独立 partition；导航闸门只允许本 origin |
| 请求信封 | `{abi:"zeokeel-app/1", app_id, version?, method, path, query, headers(白名单), body_b64, request_id}`；客户端补 `Origin` 与 Bearer，**app 页面永远看不到 token** |
| 帧 ABI（服务端↔guest） | p1 core module：guest 导出 `_start`；请求/响应用 `RS<len>\n<JSON>` 帧（长度前缀 + 单行 JSON）；编码只用 JSON，语言无关；同一套 **golden frame conformance** 测试校验 Go/Rust 两种 SDK |
| 宿主函数（封闭） | `db.exec`、`db.query`、`log`、`asset.read`（资源名→bytes）；无 `ai.*`、无 socket、无文件、无 env、无时钟配置；新增 host function 视为 ABI 变更，需版本化 |
| 语言 Tier | Tier-1：Go 1.27 wasip1、Rust 1.99 wasip1；预览：TinyGo；C/C++ 仅 vendor；wasip2 与其他语言暂缓（`research/wasm-languages.md`） |
| 服务端入口 | `POST /api/client/v1/apps/{app_id}/request`；身份只由 Bearer 推导，信封中任何用户字段不可作为授权依据；未登录/未授权 ⇒ 401/404 |
| 响应 | `{status, headers(白名单，去 Set-Cookie/逐跳头), body_b64}`；非信封响应/畸形 URL/超限 ⇒ 可读错误页（fail-closed），不是白屏 |
| DB 边界 | 宿主函数仍只有 `db.exec`、`db.query`、`log`、`asset.read`；无 `db.define`/`tx_*`/文件/网络/env/时钟配置；单语句、仅 SELECT/INSERT/UPDATE/DELETE，禁 DDL/ATTACH/PRAGMA；每应用一个 SQLite |
| 数据隔离 | 应用之间靠库文件边界；**应用内不按用户隔离**（同应用共享一库，需要按人区分由应用自加字段） |
| AI | **应用不可调 AI**：ABI 无 `ai.*` 宿主函数；`__zeokeel/ai/*` 一律 404；应用不能以"应用身份"调用网关（归因不可信，不得用于授权/对账） |
| 资源与超时 | 请求 1 MiB / 响应 8 MiB；服务端端到端 60 s（含排队）、guest 30 s、单条 SQL 5 s；每应用/每用户/全局并发限额；超限 429 + Retry-After，客户端超时略大于服务端 |
| 错误映射 | 服务端 HTTP 错误 → 应用可见的 status+code（APP_DELISTED/APP_UNAVAILABLE/429/5xx）；客户端转发层不吞错、不改写业务错误 |
| 缓存 | 客户端只缓存静态子资源，键 = 会话作用域 + app_id + version + path；登出清空；应用数据不落客户端 |

## 10. 契约生成与漂移门禁

| 产物 | 生成者 | 消费者 | 门禁 |
| --- | --- | --- | --- |
| `shared/contracts/ts/contracts.ts`（interfaces + `SCHEMAS`） | `cargo test export_schema` + `scripts/schema-ts.py` | 客户端 UI 与 server-connector（Ajv） | `git diff --exit-code` + `scripts/schema-check.sh`；枚举 tagging 与 serde 一致 |
| `http-routes.json`、`permissions.json` | `zk api` 从 `[[routes]]` 生成 | 客户端权限门控、文档、全量扫描测试 | `--check`；路由/权限/幂等/审计/限流/body 上限缺一即失败 |
| `openapi.json` | 路由表 + schemars | 内部联调/可选对外 | 生成与入库默认开启、`--check`（已定）；是否对外发布待定（15 §C；§11 04-Q5） |
| `events.json` + topic 注册表 | `zk-contract-events` | 双端 | 未注册 topic/type ⇒ CI 失败；缺 schema_version ⇒ 失败 |
| `shared/contracts/schema/*.json` | schemars（`export_schema`） | 工具/配置校验、前端 Ajv 与 TS 生成 | `git diff --exit-code` |

**实现状态（M2）**：`GET /api/meta/capabilities`（ETag）与 `/api/admin/meta` 已提供版本/特性/权限矩阵；
`GET /config/bootstrap` 已下发身份/权限/模型目录/技能/连接器/应用目录/品牌/更新清单，`config_etag`+ETag/304，
低于 `ZEOKEEL_MIN_CLIENT_VERSION` 返回 426 `CLIENT_TOO_OLD`，目录/模型加载失败返回 503 `CONFIG_UNAVAILABLE`；
`GET /api/channel` 已提供渠道/品牌/资产基址/更新策略（ETag/304）。`/api/updates/manifest` 已提供 ETag/304、
`min_supported`/`forced`（服务端设置或发布元数据）、签名状态（配置 ed25519 公钥时 fail-closed）、下载 Range/206/416；
业务面全局 426 层已实现（`/api/client/v1/*`，登录/刷新/bootstrap 例外；缺失/非法 `X-ZeoKeel-Client` fail-closed），支持渠道级最小版本与 `X-Request-Id` 回显；客户端统一发送客户端标识（真实平台）并展示强制升级门禁（版本说明 + 门户，不重试业务请求）；真实签名密钥/证书仍 UNVERIFIED（见 §9）。
access JWT 含 iss/aud/uid/role/epoch/device/channel/jti 并在服务端校验；refresh 绑定 device+channel；
admin 会话 TTL/idle 由设置键 `admin.session_ttl_secs`/`admin.session_idle_secs` 控制。

规则：DTO 只写在契约 crate（双端共用，禁止两侧各写一份）；网络 DTO 不加 `deny_unknown_fields`，保证旧客户端可解析新服务端；枚举增值客户端降级 Unknown 并保留原文；`errors.json` 由错误码枚举生成；契约 PR 必须附 `cargo public-api` diff 与两侧反序列化对拍样本。

## 11. 实现细节待定（索引 15 §C；编号 04-Q* 保留）

> 本表给的是**默认值**，不是已定结论；实现前在「收敛点」指定的里程碑 change spec 冻结（C16）。
> 未决项统一索引见 [`15` §C](15-open-questions.md#c-待定清单) T14（唯一正文在本表，15 只引用）。
> 编号缺 Q1/Q3/Q10/Q12：已决或并入 D，不再列出。

| # | 问题 | 影响 | 默认值 | 收敛点 |
| --- | --- | --- | --- | --- |
| 04-Q2 | refresh 寿命/轮换与 reuse 检测处置（access 已定 15 min） | 安全 vs 离线体验；401 风暴 | refresh 30 d / 轮换 + 族吊销 | M2 spec |
| 04-Q4 | 强制更新与离线宽限（到期限后是否允许纯本地会话；与 `15` §C T8 同族） | 更新契约、用户可用性 | 允许仅本地 agent，禁服务端依赖功能 | M2 spec |
| 04-Q5 | OpenAPI 是否对外发布/作为第三方集成契约 | 契约范围与兼容承诺 | 只内部用，不承诺外部 | M2 spec |
| 04-Q6 | WS 鉴权用一次性 ticket 还是首帧鉴权；是否复用本地 IPC 通道 | 事件协议、安全、复杂度 | ticket（浏览器/WebView 不便自定义头） | M2 spec |
| 04-Q7 | 灰度口径（按渠道/用户比例）与是否允许用户自选通道 | 版本策略 | 服务端配置 + 渠道，用户不可自选 | M3 spec |
| 04-Q8 | 幂等键的适用范围（网关写、技能发布、遥测） | 错误模型与重试 | 仅显式声明幂等的端点 | M2 spec |
| 04-Q9 | 个人版本地 MCP/连接器形态（BYOK 已由 D7/D8/C7 定） | 配置下发、网关边界 | 个人版本地自配（D53：M3 后补）；企业版关闭 | M3 spec |
| 04-Q11 | step-up 二次确认 token 的形态与有效期（header vs 独立端点、绑定参数摘要） | 写端点安全与可用性 | `X-ZeoKeel-Confirm` 一次性 60 s，绑定 operation + 参数摘要 | M2 spec |
