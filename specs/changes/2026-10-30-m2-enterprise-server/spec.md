# 2026-10-30-m2-enterprise-server · 规格

> 领域规则以 `07` 为准；进程/DB/认证/配置/管理台会话以 `06`、`04`、`20`、`21`、`22` 为准。
> 本 spec 只冻结 M2 切点；技能/连接器完整中心属 M3，不在 M2。

## 1. 进程与配置

- 单进程 `zk-server`（axum 0.8 + tokio + PostgreSQL 16）；启动 fail-closed。
- 环境键**完全按 `docs/06` §5 / `docs/09` 部署节**（`.env` 六键 + 可选 IdP/上游），不另造键名。
- `/healthz`（进程）、`/readyz`（迁移完成）；SIGTERM 优雅关闭（`06` §6/§8）。
- 迁移：`zk db lock` 分配全局序号 + 内容哈希 + `schema_migrations`；不可变、只前向、expand/contract（`24` §5）。
- 路由/权限：manifest `[[routes]]` + `zk api` 生成 catalog/permissions；缺声明拒启（`24` §4）。

## 2. 认证 / 客户 API

- 客户端 API 前缀按 `04`：`/api/client/v1/auth/{login,refresh,logout,methods,me,oidc/exchange}`；不得用 `/api/auth/*` 另起前缀。
- 认证源：local / OIDC / LDAP 单一主源 + 应急超管；`/auth/methods` 返回当前源可用的登录方式。
- JWT：EdDSA + `kid` 轮换；access 15m、refresh 30d 旋转；`token_epoch` 每请求校验；refresh 重用检测 → 全链吊销。
- token 绑定 `device_id` + `channel`；禁用/改密/登出递增 epoch（`04` §3、`21`）。
- 表字段按 `04` §3 + `07` §2：`users/user_devices/refresh_tokens`；认证源配置来自 env/`system_settings`，不建 `auth_sources` 表；新增字段 `serde(default)`。

## 3. 管理台认证（P0）

- 管理台**独立 Cookie session**，不复用员工 JWT：`/api/admin/auth/login` 账号密码 + 强制 CSRF token；`/admin/*` 只托管静态页；
  员工 JWT 调 `/api/admin/*` → 401/403；auditor 只读；`super_admin` 可写；失败锁定 + 审计（`06` §4、`04`、`21`）。
- Cookie：`HttpOnly; Secure（生产）; SameSite=Lax; Path=/`；CSRF token 单独随机生成、服务端 session 保存、常量时间比较、请求头 `X-CSRF-Token`；跨源拒绝；登录/退出/写操作全校验；失败锁定/审计。
- 管理台 MFA/锁定按 `21` 配置；无 MFA 时明确为内部评审风险（D65）。

## 4. 用户 / 钱包

- 用户/角色按 `07` §2：`super_admin`/`auditor`/`employee`，无部门/用户组；可见性全员/指定用户。
- 钱包/价格**引用 `22` 的完整模型**（holds/结算/对账/峰谷/月发放/幂等键按 22）；本 spec 不另造表名，pending usage 是状态/清扫机制而非新表。
- 扣费并发：`SELECT … FOR UPDATE` + hold；超额硬阻断；流式允许在配置上限内透支；金额 i64 micros，checked/saturating。

## 5. AI 网关

- 只 `POST /v1/responses` + `GET /v1/models`；上游 OpenAI-compatible Responses；SSE 逐块转发。
- `providers/models` 表 + 模型定价/上游路由；未定价/未知模型拒绝（封闭错误码，`07` §4、`22`）。
- 上游 key 只在服务端；SSRF 白名单（`21`）；请求前余额检查；流式结束按 usage 结算（`pending usage` 机制，`22`）；
  上游取消/超时按已用 token 结算；usage + ledger + balance 同事务。
- 工具调用只透传，网关不执行工具/agent。

## 6. 目录 / 审计 / 管理台

- M2 只做 **模型目录 + 更新通道 + bootstrap**（`GET /api/client/v1/config/bootstrap`）；商店目录可为空只读；
  **技能/连接器 CRUD/发布/审批/可见范围属 M3**，M2 不做。
- 审计：`audit_log` 含 `prev_hash/hash/request_id/ip/device/actor/action/target/before/after`，哈希链 + WORM 语义、≥180 天 fail-closed；用量 ≥12 个月（`21` §8）。
- 管理台页面按 `07` §9.3：`admin-shell/users/wallet/pricing/gateway/usage/audit/settings/ui-kit`；M2 启用除 skills/apps/connectors 外的页面；登录页/CSRF 归 `admin-shell`。
- 所有写操作审计；配置修改标注即时/重启。

## 6.1 M2 路由/权限目录（冻结草案，`zk api` 校验）

| 方法与路径 | 权限 | 说明 |
| --- | --- | --- |
| `GET /api/meta/capabilities` | public | 版本/能力协商 |
| `GET /api/client/v1/auth/methods` | public | 认证源 |
| `POST /api/client/v1/auth/login` | public | 密码/OIDC/LDAP 会话 |
| `POST /api/client/v1/auth/refresh` | refresh | 旋转/重用检测 |
| `POST /api/client/v1/auth/logout` | bearer | 吊销当前设备 |
| `GET /api/client/v1/auth/me` | `auth:self` | 用户/权限/设备 |
| `POST /api/client/v1/auth/oidc/exchange` | public(one-time) | SSO 深链 |
| `GET /api/client/v1/config/bootstrap` | `config:read` | 模型/商店/更新目录（空数组允许） |
| `GET /api/client/v1/wallet` · `/ledger` | `wallet:self` | 余额/流水 |
| `POST /v1/responses` | `gateway:chat` | 唯一 AI 出口 |
| `GET /v1/models` | `gateway:chat` | 可用模型/展示价 |
| `POST /api/admin/auth/login` · `/csrf` · `/logout` | admin cookie+CSRF | 管理台会话 |
| `GET/POST /api/admin/users*` | admin users | 用户/角色 |
| `GET/POST /api/admin/wallet*` | admin wallet | 充值/调整 |
| `GET/POST /api/admin/pricing*` | admin pricing | 峰谷价 |
| `GET/POST /api/admin/gateway*` | admin gateway | 上游/模型 |
| `GET /api/admin/usage*` | admin usage | 用量 |
| `GET /api/admin/audit*` | admin audit | 审计（auditor 可读） |
| `GET/PUT /api/admin/settings*` | admin settings | 系统配置 |

## 7. 企业客户端

- 企业 edition：exclude 个人 provider 设置 UI/GitHub 更新器；新增 `client-auth`、`client-bootstrap`、`llm-gateway`、`client-update-server`。
- 登录/refresh/未登录态；bootstrap 下发模型/商店/更新目录；网关 provider 不可配 base_url/key。
- 构建期裁剪验证：dist/设置快照不含被禁字段；员工不能本地伪造目录/绕过网关。

## 8. 部署

- 多阶段 Dockerfile + compose（server/postgres/caddy/portal）+ 备份/恢复/升级/回滚 runbook（`20`）；
- master key 与 DB 备份分离；`docker load` 升级；`.env` 按 `06`/`09`；`/metrics` 可选；健康检查 fail-fast。
