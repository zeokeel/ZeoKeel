# zk-srv-connectors (`srv.connectors`)

**功能说明（M3）**：MCP 连接器聚合。

- 定义/grants/OAuth 公共客户端元数据；不含静态 secret（ADR-060）。
- 管理员创建/下发/grants/oauth/force-update/软删。
- 员工目录按 grants；启停写 `connector_prefs`（只影响本人）。
- 服务端不代理 MCP；OAuth 授权码/PKCE/令牌在客户端本地。
