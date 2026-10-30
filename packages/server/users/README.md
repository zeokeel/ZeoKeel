# zk-srv-users（server domain）

**功能说明（M2）**：用户/角色 CRUD、auditor 只读、软删。

## 边界
- 只依赖 `zk-kit-server` + 域契约；不 import 其它服务端域实现。
- 路由以 `routes() -> Router<AppState>` 暴露，由 `apps/server` 组合。
- 数据库表/迁移由本域负责（后续从 srv-http 迁入）。

## 状态
- 目录/清单/conformance 已建立；业务代码正从 `packages/server/http` 迁入。
