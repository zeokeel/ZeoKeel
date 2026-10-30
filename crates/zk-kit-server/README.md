# zk-kit-server

服务端共享基础设施（**不是业务域包**）：`AppState`（PgPool + JWT key）、JWT 签发/校验、
argon2 密码、Cookie/CSRF、`require_admin`、审计写链、统一错误。

业务规则/路由在 `packages/server/<domain>`；域包只依赖本 kit 与契约，不互相依赖。
