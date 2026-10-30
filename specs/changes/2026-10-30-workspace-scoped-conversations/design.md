# 2026-10-30-workspace-scoped-conversations · 设计

## 1. 为什么 SQLite 单 store

工作区与会话共享事务（remove/reassign、迁移、默认唯一），单 SQLite/单 writer 最小；不新增 `store.workspace` 包，`store-session-sqlite` 同时实现两个 trait。
角色注册仍为 `store.session` / `zk-contract-session`。

## 2. 迁移顺序（不可逆，先备份）

1. `PRAGMA wal_checkpoint(TRUNCATE)`；复制 db/-wal/-shm。
2. `PRAGMA foreign_keys=OFF`；BEGIN；建 workspaces/默认行；重建 conversations_new/messages_new；copy；drop 旧表；rename；索引；user_version=2；COMMIT；`foreign_keys=ON`。
3. 校验行数/孤儿/默认唯一；失败恢复备份，UI 只读。

## 3. 默认 Workspace

- 路径：`dirs::home_dir()`（用户 home `~`）；canonicalize 后写入；home 不可用 → 启动时返回错误让 UI 选择目录（不猜）。
- 侧栏 Workspace 列表平铺（不嵌套/不分组）；默认 `~`。
- 默认 id 由迁移生成 UUIDv4；已有默认不重复插入。

## 4. 兼容层

旧 `create_conversation/list_conversations/messages` 标 `#[deprecated]`，内部调用 `*_in(default_workspace)`；bridge 暂不破坏。
T3/T4 改为全部 workspace-scoped 并删除旧方法；C18 由 IPC 边界强制。

## 5. UI 设计（T4）

高密度三栏原型已交用户评审；通过后按 docs/27 更新密度 token，再实现 Workspace switcher/首用/切换取消 turn。
设计未批准前不改 UI 功能。
