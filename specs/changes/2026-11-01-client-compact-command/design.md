# 2026-11-01-client-compact-command · 设计

> 行为见 spec；状态 draft，D1/D2/D6 已定（Lead 2026-11-01）。

## 0. 共享设计（三子 spec 一致）
- **串行**：compact → plan-mode → skill-commands；共享文件（`contracts/ipc`、`ui-chat/AssistantComposer`、`App.tsx`、`controller.ts`、`i18n.tsx`、`e2e/m1-serve`）由先实现者（compact）写，后序 rebase 追加。
- **错误码**：本 spec 新增 `compact/*`；docs/26 §5 登记；`command/unimplemented` 对 compact 移除。
- **契约/生成物**：`zk-contract-ipc` additive；schema/TS 由生成器写；无新 capability/DB migration；菜单占位由 Batch1 catalog 改真实。
- **不做**：plan/skill 后端（另 spec）；自动压缩；sandbox/企业策略。

## 1. 影响面
| 类型 | 变更 |
| --- | --- |
| 契约 | `session.compact`/`session.compaction_get` + DTO/错误码；不改 ChatSendPayload/session 表 |
| 存储 | 无 migration；marker 复用 `append_message`；**`messages_in` 原始语义不变**；compact view 在 host 从 raw+marker 构造 |
| 包 | `zk-kit-compaction`（新 kit；专用 summarizer）；ui-chat CompactDivider |
| 文档 | docs/19、docs/26、KNOWN（docs/38 只读参考） |

## 2. 方案

### 2.1 summarizer kit
```rust
pub struct CompactionRequest<'a> { history: &'a [Message], keep_recent: usize, max_input_tokens: usize, max_summary_tokens: usize }
pub struct CompactionResult { summary: String, through_message_id: String, input_tokens: u64, output_tokens: u64 }
pub fn summarize(provider: &ProviderConfig, request: CompactionRequest) -> Result<CompactionResult, CompactionError>
```
- personal 用 `AppSettings.provider`；enterprise 用 `zk_composed::gateway_credentials()` + gateway model（C7），不持上游 key。
- 输入取 `messages_in` 视图最近 N 轮之前的 user/assistant 文本（tool 行作纯文本摘要）；用既有 token 估算；超 12k 拒绝；prompt 保留目标/决策/文件/未完成项；摘要 ≤2k，失败不返回部分结果。
- 不创建 agent turn/tool 行/审批；不写日志内容。

### 2.2 host `/compact`
- IPC `session.compact {workspace_id, conversation_id}` → `{marker_id, summary_preview, compressed_messages, input_tokens, output_tokens, noop?}`；`session.compaction_get` → 最新 marker 或 null。
- 流程：归属校验 → streaming 拒绝 → 读当前视图 → 计算边界/无新消息 no-op → summarize → append marker system message → 返回；任何失败不 append。
- marker content JSON 含 `summary/through_message_id/created_ms/input_tokens/output_tokens/keep_recent`；id `compaction:<uuid>`（uuid 新生成）。
- `replay_history`：遇到 marker 时提取 summary，作为 `HistoryRole::User` 文本注入历史；不注入 marker 原文。
- 并发：同 conversation 串行（host compact mutex/active gate），重复请求以最新 marker 边界幂等。
- 保留最近 6 轮/12 条；原始消息不删。

### 2.3 compacted view（P1-1）
- **不改 store**：`SessionStore::messages_in` 继续返回原始消息；host 在 `session.compacted_view`/replay 时读取 raw，找最新 `role=system` 且 id 前缀 `compaction:` 的 marker，构造「marker + 最近 N 轮」视图（仅内存）；无 marker/损坏/未知 through 时回退 raw+提示。
- `session.messages`（UI）保持 raw；`CompactDivider` 用 `session.compacted_view` 的 marker/boundary 在 UI 插入分隔/摘要，不隐藏 raw 旧消息；`session_search`/fork/审计一律 raw（测试覆盖）。
- `append_message` 不特殊处理；无 migration；无第二套 store 语义。

### 2.4 UI
- Batch1 catalog 将 `slash-compact` 从 unimplemented 改 `kind:"execute"`；`runCommand` 调 `actions.compactConversation`；UI busy/失败/成功。
- `CompactDivider`：messages 中 marker 渲染「较早消息已压缩」+摘要可展开/压缩条数/usage；不展示 JSON；失败仅 toast/行内，不插入分隔。
- 六态/i18n/axe/焦点回 composer；流式中禁用并提示。

## 3. 备选与取舍
- agent turn 摘要（产生消息/usage/审批）→ 拒绝；改 rig history/rig-memory → 跨层/不可控；DB 表 → 不必要，marker 复用 messages 更小；自动压缩 → 另 spec。

## 4. 安全与隐私
- 摘要/原始消息用户数据，不写日志；enterprise 只网关；workspace 归属；无跨 workspace；marker 不含 key/URL。

## 5. 测试计划
| 层 | 用例 | 命令 |
| --- | --- | --- |
| kit | 输入截断/超限/摘要上限/provider 失败不写 | `cargo test -p zk-kit-compaction` |
| host view | compacted view 构造/回退/原始回归；搜索/fork/审计 raw | `cargo test -p zk-desktop` |
| host | compact 成功/noop/流式/失败/归属/replay 注入 | `cargo test -p zk-desktop` |
| UI/app | 菜单/分隔/六态/键盘；compacted view 仅分隔/replay；search/fork raw | `pnpm --dir packages/ui/ui-chat/web test:unit`；`pnpm --dir apps/desktop/web test:unit` |
| e2e | `/compact` 成功→下一轮摘要；失败不改历史；流式禁用；axe | `pnpm e2e`；`e2e/interaction/command-compact.spec.ts` |
| 门禁 | schema/compose/i18n/交互 | 见 tasks §4 |

## 6. 文档
- docs/19 compact 分隔/失败/流式；docs/26 marker/IPC/错误码/compacted view；KNOWN 无自动压缩/原始仍占 DB；Lead 12 文件例外。

## 7. 风险
| 风险 | 缓解 |
| --- | --- |
| marker 视图破坏 fork/export | store 测试/fork/原始保留 |
| provider/网关失败 | 不 append、可重试、usage/错误可读 |
| 摘要 token/费用 | 12k/2k 上限、显式触发、usage 可见 |
