# 2026-10-30-workspace-scoped-conversations · 规格

## 1. 契约（zk-contract-session）

```rust
pub struct Workspace { id, path, title, is_default, created_at_ms, updated_at_ms }
pub struct Conversation { id, workspace_id, title, created_at_ms, updated_at_ms }
pub struct RemoveWorkspaceResult { removed: bool, moved_sessions: u64 }

pub trait WorkspaceStore {
  fn list_workspaces(&self) -> Result<Vec<Workspace>, SessionError>;
  fn create_workspace(&self, path: &str, title: &str) -> Result<Workspace, SessionError>;
  fn rename_workspace(&self, id: &str, title: &str) -> Result<Workspace, SessionError>;
  fn set_default_workspace(&self, id: &str) -> Result<Vec<Workspace>, SessionError>;
  fn remove_workspace(&self, id: &str, reassign_to: Option<&str>, confirm: bool) -> Result<RemoveWorkspaceResult, SessionError>;
}

pub trait SessionStore {
  // 新 API（M1 权威）
  fn create_conversation_in(&self, workspace_id: &str, title: &str) -> Result<Conversation, SessionError>;
  fn list_conversations_in(&self, workspace_id: &str) -> Result<Vec<Conversation>, SessionError>;
  fn messages_in(&self, workspace_id: &str, conversation_id: &str) -> Result<Vec<Message>, SessionError>;
  // 旧 API：临时兼容 bridge，T4 删除（不得用于新代码）
  #[deprecated] fn create_conversation(&self, title: &str) -> ...
  #[deprecated] fn list_conversations(&self) -> ...
  #[deprecated] fn messages(&self, conversation_id: &str) -> ...
}
```

错误：`WorkspaceNotFound`、`WorkspaceMismatch`、`DuplicateWorkspacePath`、`WorkspacePathUnavailable`、`WorkspaceInUse`、`InvalidWorkspace`、`ConfirmRequired`。

## 2. 存储

- schema v2（DDL 见 docs/26 §3）；启动 `user_version` 迁移；旧库默认 Workspace 回填；失败回滚/备份/只读。
- 默认 Workspace 恰好 1 个；`set_default` 原子切换；默认不可移除；移除有会话需 `reassign_to` + `confirm`。
- `create_workspace` 要求 path 已存在目录/可读写，canonicalize 唯一；同 path 已登记 → Duplicate。
- 旧 API 使用默认 Workspace（仅兼容层，T4 删除）。

## 3. Agent/tool（T2）

`TurnContext { conversation_id, workspace_id, workspace_root }` 定义在 `zk-contract-tool`；`Agent::run/run_stream`、`Tool::call` 接收；
`tool-fs` 由进程 root 改为 per-call root；未绑定 workspace = `WorkspaceRequired`。

## 4. IPC（T3，设计评审后 T4 UI）

命令表见 docs/26 §5；所有 session/chat 带 workspace_id；删除 auto-create；错误码封闭集。

## 5. 测试

- 迁移：v1 会话+消息 → v2 默认归属，消息不丢；失败注入回滚；幂等。
- workspace：create 重复/不存在/不可写；rename；set_default；remove 无会话/有会话无 reassign/有 reassign；默认移除拒绝。
- session：创建归属；list 隔离；messages mismatch；旧 API 兼容测试（标记 deprecated）。
- E2E（T4）：首用/切换/隔离/重启/目录不可用 + axe。
