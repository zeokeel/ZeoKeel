# 2026-10-31-round4-data-and-ui · 规格

- 状态：`approved`

## 1. Usage 持久化

- `zk-contract-session::Message` 新增 `#[serde(default)] usage: Option<zk_contract_agent::AgentUsage>`（会话契约依赖 agent 契约，无环）；schema/TS 重新生成。
- `store-session-sqlite` schema v4：`messages` 增加 `usage_json TEXT`（NULL 兼容旧行）；append 序列化、messages_in 解析（损坏忽略并告警）。
- 桌面 chat.send 完成/取消/错误路径写入 usage（错误/取消无 usage 则 NULL）；UI 从消息恢复 usage 展示。

## 2. Settings 分 section

- `RuntimeSettings { #[serde(default)] agent: Option<AgentSettings>, #[serde(default)] tools: Option<ToolSettings> }`。
- 桌面仅在设置文件存在且对应 section 非默认时发布该 section；旧 snapshot（全量）仍可解析。
- agent-rig 每 section 独立合并：published section > legacy registry/env > 默认；测试覆盖 tools-only + env agent、agent-only + env tools。

## 3. workspace.changes

- `zk-contract-ipc::results::WorkspaceChange { path, add: u32, del: u32 }`、`WorkspaceChangesResult { items: Vec<WorkspaceChange> }`；schema/TS 生成。
- 桌面实现 `workspace.changes`：解析 workspace，运行 `git -C <root> status --porcelain -z`（无 git/非仓库返回空 items+warning），解析 XY 状态为 add/del 计数（解析限制/上限 500 项）；只读、超时 5s。
- Web：inspector 变更 tab 启用，加载 changes（空/加载/错误态），兼容无 git；docs/27/28 更新。

## 4. T22 清理

- 桌面 host 返回 JSON 改用生成的 DTO 形状（`WorkspaceFilesResult`/`MetaInfo`/`ChatSendResult`）；question_asked 校验使用 `QuestionAskedPayload`；FileEntry TS 镜像改为生成类型。

## 5. 企业交付复核

- `scripts/updates-e2e.sh`、`scripts/oem-e2e.sh`、portal/admin e2e 在当前代码上运行；失败修复或记录 UNVERIFIED。
