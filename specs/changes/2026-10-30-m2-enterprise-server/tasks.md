# 2026-10-30-m2-enterprise-server · 任务

| # | 任务 | 写作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | P1 HTTP/DB/迁移骨架 + health/ready + zk db lock + routes/zk api | packages/server/http, apps/server | — | cargo test + PG + `zk db lock --check` + `zk api --check` | ☐ |
| 2 | P1 client auth（local/OIDC/LDAP + methods/me/refresh/epoch/device） | packages/server/auth-{local,oidc,ldap,session}; contracts/auth-client | 1 | API 集成/负向 + `zk api --check` | ☐ |
| 3 | P1 admin cookie session + CSRF + auditor 只读 + 锁定 | packages/server/auth-session; apps/admin | 2 | 权限/CSRF e2e | ☐ |
| 4 | P1 users/roles/admin API | packages/server/users, admin-users | 3 | 权限矩阵 | ☐ |
| 5 | P2 wallet/ledger/holds/pricing/峰谷/月发放 | packages/server/{wallet,pricing} | 4 | 并发/对账 | ☐ |
| 6 | P2 gateway /v1/responses + /v1/models + usage/pending/结算 | packages/server/gateway | 5 | 流式/超额 e2e + `zk api --check` | ☐ |
| 7 | P3 bootstrap/审计/admin 页面 | packages/server/{http,audit}; admin-* | 3,6 | 下发/审计 e2e | ☐ |
| 8 | P4 企业客户端 login/bootstrap/gateway/update | packages/client/{auth,bootstrap,gateway,update}; apps/desktop | 2,6 | e2e/裁剪 + enterprise compose --check | ☐ |
| 9 | P4 docker/compose/caddy/portal/backup/runbook | deploy/, scripts | 1-8 | 新环境演练 + enterprise compose --check | ☐ |
| 10 | 对抗性验收（安全/CSRF/并发/权限/升级回滚） | 只读 | 1-9 | 0 P0/P1 | ☐ |
