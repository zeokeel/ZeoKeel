# zk-client-auth (`auth.client`)

**功能说明（M2 P4）**：企业客户端认证客户端。

- `login(server_url, username, password, device_id)` → `AuthSession`
- `refresh(session)` → 新 access/refresh
- `me(access_token)` → user/role
- `logout(access_token)` → 吊销
- 只做 HTTP/令牌状态；UI 登录页与 token 持久化在客户端后续接入。
- 不持有上游 provider key；企业 AI 只走 gateway。
