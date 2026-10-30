# zk-srv-http (`srv.http`)

**功能说明（M2 P1）**：企业服务端 HTTP 入口与数据库就绪检查。

## 职责
- 构建 axum Router：`/healthz`（进程）、`/readyz`（PostgreSQL `SELECT 1`）、`/api/meta`。
- `connect(database_url)`：PgPool；`run_migrations`：应用不可变 SQL 并写 `schema_migrations` 校验和。
- 启动 fail-closed：数据库不可用/迁移失败则进程退出；SIGTERM 优雅关闭由 `apps/server` 负责。
- 网关（M2 P2 部分）：仅 `/v1/responses`；未定价/余额不足拒绝；上游 key 只在服务端；usage + 账本同事务结算。
- 价格/模型目录（M2 P2 部分）：admin 峰谷倍率/单价 CRUD、client `/v1/models`。
- 钱包（M2 P2 部分）：client `/wallet`/ledger、admin 充值/调账（幂等、行锁、账本派生余额）。
- 认证（M2 P1）：local 登录/EdDSA JWT/refresh/me/logout；admin Cookie+CSRF；users/roles CRUD（auditor 只读）。
- 路由/权限 catalog（`[[routes]]`/`zk api`）与认证/管理台中间件在 P1 后续任务中加入；本包只做入口地基。

## 依赖
`zk-contract-server`（角色/描述符）、axum、sqlx、tower-http。不依赖任何客户端/UI 实现。
