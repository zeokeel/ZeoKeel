# @zeokeel/ui-settings-mcp (`ui.slot.settings-mcp`)

**功能说明**：个人版本地 MCP server 设置 pane（`ui.settings.pane` order 70）。

- 列表/新增/编辑/删除/测试连接；`settings.set` 由外壳统一保存（本 pane 只写草稿 `mcpServers`）。
- 测试连接走 app 注入的 `testMcpServer`（`mcp.test` + `mcp.test.status` 轮询；无缓存探测）。
- 非 secret 配置；`oauth_required` 本地不允许（Rust 校验 + 前端基础提示）。
- `editions = ["personal"]`：enterprise 构建期 exclude（C1/C2，不做运行期分支）。
