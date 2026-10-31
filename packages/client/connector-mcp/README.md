# zk-connector-mcp

**功能说明**：MCP（Model Context Protocol）客户端 transport（B16/R9.6）。把已配置的 MCP server 工具
通过既有 agent 工具注册/glue 机制暴露给本地 agent（构建期组合，无运行期插件加载，C2）。

- 传输：`stdio`（spawn 配置的 command + 逐行 JSON-RPC；`env_clear` 后只注入 allow-list env）与
  明文 `http://` POST（JSON 或 `text/event-stream` 解析；HTTPS/TLS **UNVERIFIED**）。
- JSON-RPC：`initialize` + `notifications/initialized`、`tools/list`、`tools/call`；每次请求有超时；
  子进程在 transport drop 时 kill + reap（无僵尸）；不打印/记录 token 或 env 值。
- 配置：`zk-contract-connector::McpServerConfig`（非 secret；ADR-060）。运行期来源为企业 catalog
  （`update_from_catalog`，desktop 在登录/`connectors.configure` 后安装）或本地 `ZEOKEEL_MCP_SERVERS`
  （JSON 数组）；`update_from_catalog`/`clear_catalog_config` 是 force-update/登出入口。
- 诚实不可用：未配置/找不到 server/不可达时返回 `ToolError::Denied`（`MCP_CAPABILITY_UNAVAILABLE`），
  不静默成功。
- 暴露工具：`mcp_call`（server/tool/arguments）+ `mcp_list_tools`（extra_tools；generator 检测
  `pub fn extra_tools` 后自动注册）。
- **OAuth（R9.6，客户端公共客户端 + PKCE）**：授权码/PKCE 流程由 desktop host 实现
  （`apps/desktop/src/mcp_oauth.rs`：127.0.0.1 临时端口 loopback redirect + 系统浏览器 + token 交换/刷新，
  token 经 keychain IPC 保存）。本包只负责注入：
  `set_oauth_token(server_id, Option<String>)` / `clear_oauth_tokens()` / `has_oauth_token()`——
  仅对 `McpServerConfig.oauth_required` 的 server 在每次 HTTP 请求加 `Authorization: Bearer <token>`
  （内存持有，无 Debug/日志）；无 token ⇒ `MCP_OAUTH_REQUIRED`（不发请求、诚实不可用）；非 OAuth server
  永不携带该头；`update_from_catalog` 只保留仍存在的 connector token，`clear_catalog_config` 清空
  （登出/断开路径）。端点校验：https，http 仅 loopback（见 `docs/07` §6.2）。

**依赖**：`zk-contract-connector`、`zk-contract-tool`。

**测试**：`cargo test -p zk-connector-mcp`（16 tests：stdio fake MCP server list/call/timeout/malformed/
nonzero-exit/config 校验；HTTP fixture JSON/SSE；TLS ClientHello；OAuth 注入/清除/`MCP_OAUTH_REQUIRED`/
catalog retain；无网络外部依赖）。

**验证**：`zk validate`、`zk compose --edition personal --profile default --channel official --check`、
`cargo clippy -p zk-connector-mcp --all-targets -- -D warnings`。
