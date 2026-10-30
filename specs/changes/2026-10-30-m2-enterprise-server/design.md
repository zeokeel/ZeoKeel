# 2026-10-30-m2-enterprise-server · 设计

## 1. 包/契约（C3/C4）

- 唯一真源：`02` §5、`07` §9、`08` §3、`24` §1.5。下表是 M2 **实际启用/登记**名单，不另造名称。
- 服务端域：`zk-srv-http`、`zk-srv-auth-local`、`zk-srv-auth-oidc`、`zk-srv-auth-ldap`、`zk-srv-auth-session`、
  `zk-srv-users`、`zk-srv-wallet`、`zk-srv-pricing`、`zk-srv-wallet-grant`、`zk-srv-gateway`、
  `zk-srv-gateway-upstream-openai`、`zk-srv-gateway-meter`、`zk-srv-audit`、`zk-srv-settings`、`zk-srv-jobs`。
- 管理台页面：`admin-shell`、`admin-users`、`admin-wallet`、`admin-pricing`、`admin-gateway`、`admin-usage`、
  `admin-audit`、`admin-settings`、`admin-ui-kit`；`admin-skills/apps/connectors` 是 M3 包，M2 不启用。
- 客户端：`client-auth`、`client-bootstrap`、`llm-gateway`、`client-update-server`（docs/03 的企业更新源包名）。
- 契约（按 `02` §5 / `07` §9.1 / `24` §1.5）：
  `zk-contract-api`、`-events`、`-auth`、`-user`、`-wallet`、`-gateway`、`-server`、`-auth-client`、`-admin-ui`、`-update`；
  `-content`/`-connector` 属 M3。
- 包 → 契约 → 角色（C4）：

| 包 | 契约 | 角色 |
| --- | --- | --- |
| `zk-srv-auth-local/oidc/ldap/session` | `zk-contract-auth` | `srv.auth.source` / `srv.auth.session` |
| `zk-srv-users` | `zk-contract-user` | `srv.users` |
| `zk-srv-wallet` / `-pricing` / `-wallet-grant` | `zk-contract-wallet` | `srv.wallet` / `srv.pricing` / `srv.job.wallet-grant` |
| `zk-srv-gateway` / `-upstream-openai` / `-meter` | `zk-contract-gateway` | `srv.gateway` / `srv.gateway.upstream` / `srv.gateway.meter` |
| `zk-srv-http/settings/jobs/audit` | `zk-contract-server` | `srv.http/settings/jobs/audit` |
| `client-auth` | `zk-contract-auth-client` | `auth.client` |
| `client-bootstrap` | `zk-contract-api` | 角色待 T2 登记（M2 spec 前补 `client.bootstrap` 或复用 `client.config`；不擅自占用未定义角色） |
| `llm-gateway` | `zk-contract-llm` | `llm.provider.default` |
| `client-update-server` | `zk-contract-update` | `client.update` |
| `admin-*`（页面） | `zk-contract-admin-ui` + `-user/-wallet/-gateway` DTO | `admin.page.<page>`（如 `admin.page.users`） |

- bootstrap 路由由 `zk-srv-http` + `zk-srv-settings` 提供，不建 `srv-bootstrap` 包。

## 2. 请求管线

- `/api/client/v1/*`：request_id → CORS（仅客户端来源）→ JWT（员工）→ rate limit → body limit → permission → audit → handler。
- `/api/admin/*`：独立 admin Cookie session + CSRF + 角色权限（auditor 只读）；员工 JWT 401/403；`/admin/*` 只托管静态资源。
- `/v1/*`：员工 JWT（网关专用）→ 余额检查 → 上游；不与 admin 复用中间件。
- 每次 JWT 校验 `token_epoch`；错误码封闭；日志脱敏。

## 3. 数据/迁移

- 表/DDL 以 `04` §3、`07`、`22` 为准；M2 只新增这些表，不重复定义。
- `zk db lock` 生成全局迁移序号/哈希；expand/contract；启动校验；不得改历史。
- 管理员 bootstrap：一次性 env，写审计；成功后端点关闭。

## 4. 网关

- Responses SSE 转发；`/v1/models` 只返回已定价/已启用；未知模型拒绝。
- 上游 SSRF 白名单；key 加密存储/掩码；取消传播；usage 缺失时估算并标记 `estimated`。
- hold/pending usage/结算/对账按 `22`；余额不足请求前拒绝；流式透支上限配置。

## 5. 安全

- admin cookie+CSRF、失败锁定、MFA 可配；审计哈希链/WORM/≥180 天；usage ≥12 个月。
- 弱管控：不执行 agent/工具/技能；不承诺沙箱；服务端失败 fail-closed。
- 企业客户端构建期 exclude 个人 provider UI/更新器；CI 校验 bundle。

## 6. 阶段

P1 DB/迁移/HTTP 骨架/auth/users/admin 登录；P2 wallet/pricing/gateway；P3 bootstrap/audit/admin 页面；
P4 企业客户端/docker/portal/update；每阶段可运行、可回滚、e2e 证据；技能/连接器 M3。
