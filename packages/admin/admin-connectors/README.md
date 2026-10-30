# @zeokeel/admin-connectors (`admin.page.connectors`)

**功能说明（M3）**：管理台连接器页。

- 列表/创建（stdio/http/sse）
- grants（all/user，can_enable）
- OAuth 公共客户端元数据（client_id/scopes/redirect_uri，无 client_secret）
- 强制更新/软删
- 不展示/不存储静态 secret（ADR-060）；权限 `connectors:admin`。
