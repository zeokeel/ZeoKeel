# 35 · API Reference（M6）

> 唯一契约真源：`docs/04-contracts-and-api.md`；客户端业务面 `/api/client/v1/*`，稳定面 `/api/meta/*`、`/api/channel`、`/api/updates/*`，网关 `/v1/*`。

- 认证：Bearer JWT（access/refresh），admin cookie+CSRF；禁用用户 token_epoch 立即失效。
- 客户端域：auth/users/wallet/pricing/gateway/skills/connectors/apps/telemetry/updates。
- 管理面：`/api/admin/*`，super_admin/auditor 权限；写操作 CSRF。
- 错误：封闭错误码 + `{ok:false,error}`；限流 429 + Retry-After。
- 生成物：`shared/contracts/schema/*.json`/`ts/contracts.ts`；`schema-check` 防漂移。
