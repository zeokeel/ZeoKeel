# zk-store-session-sqlite

工作区 + 会话持久化实现：SQLite（WAL），表 `workspaces`/`conversations`/`messages`。
默认路径 `$ZEOKEEL_HOME/sessions.db`（否则系统 data 目录）；**默认 Workspace = 用户 home `~`**
（`dirs::home_dir()` canonicalize；`ZEOKEEL_HOME` 仅测试/嵌入 override，不是 UI 默认；无 Documents 路径）。
home 不可用返回 `workspace/path-unavailable`，由 UI 选择目录，不静默回退。测试用 `open_with_root` 注入临时目录。

- schema v2：`conversations.workspace_id` 必填；旧 v1 库启动时自动迁移（WAL checkpoint + 备份 + 重建表 + 回填默认 Workspace）。
- 实现 `WorkspaceStore` + workspace-scoped `SessionStore`；旧 `create_conversation/list/messages`
  是 T4 前的兼容层（T4 删除）。
- 规范见 `docs/26` §3/§3.1；测试 `cargo test -p zk-store-session-sqlite`。
