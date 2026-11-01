# 2026-11-01-client-compact-command · 提案（Batch2-A：/compact）

- 状态：`approved`
- 评审：docs/reviews/2026-11-01-client-command-batch2.md
- Owner：dsh-ref
- 日期：2026-11-01
- 关联：用户 DSH 1:1 Batch2；原父 spec `2026-11-01-client-command-backends` 已按 Lead D7 拆分并删除；本 spec 为 compact 子 spec；`target/audits/dsh-reference-inventory.md` §5；`docs/38` B8/history；`docs/19` §2.1b/§11；前置 Batch1 `2026-11-01-client-command-surface`
- 评审：n/a（新增 spec；实现前需 C19/C20 独立评审）
- 变更预算：**12 个文件**（见 §8；Lead 2026-11-01 书面批准 12 文件例外）

## 1. 背景 / 问题

Batch1 `/` 菜单中 `/compact` 是 Batch2 占位。ZeoKeel `SessionStore::messages_in` 回放历史，agent-rig 只按 token 预算裁剪最旧消息（`docs/38` B8），没有摘要/压缩状态；长会话会持续丢最旧消息而非压缩保留语义。用户要求 DSH 1:1，`/compact` 必须真实可用。

## 2. 目标

- `/compact` 从占位转真实命令（菜单指令组、键位/过滤同 Batch1）。
- host 摘要早期历史（personal provider / enterprise gateway），保留最近 **6 轮/12 条**；持久化压缩状态（system marker）；**`messages_in` 原始语义不变**，新增 compacted view 仅供 replay/分隔 UI；原始消息保留。
- 失败/超限/流式中不改历史；重复压缩 no-op；UI 分隔/usage；错误码收敛。
- C10/C11/C17/C18；C20 两文件+interaction；六态/i18n/a11y。

## 3. 非目标

- 不做自动压缩/阈值（另 spec）；不改 rig history/rig-memory API；不丢原始消息；不做企业策略/服务端端点。
- 不做 `/plan`/`/skill`（另两份子 spec）；不做 UI 菜单框架（Batch1 已有，只把占位改真实并增执行分支）。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 长会话个人用户 | 上下文将满，输入 `/compact` | 摘要分隔可见；下一轮保有摘要+最近对话；失败不丢历史 |
| 企业员工 | 长会话 `/compact` | 经网关摘要（不持上游 key）；usage/失败可读；原始会话仍可审计 |
| 键盘/读屏用户 | 键盘执行/读取分隔 | 菜单/分隔可读、焦点回 composer、axe 通过 |

## 5. 成功指标

- `/compact` 成功：摘要 marker + 最近 N 轮；原消息 DB 保留；下一轮 agent 收到摘要；UI 分隔/usage。
- 失败/超限/流式 0 次改历史；重复无新消息 no-op；无 `command/unimplemented`。
- 错误码 `compact/*` 登记；P0 测试+证据；axe 0 serious。

## 6. 风险与未知

| 风险 | 影响 | 缓解/验证 |
| --- | --- | --- |
| 摘要调用产生 agent turn/工具/审批 | 高 | 专用 host summarizer（kit），不产生工具行；测试 |
| marker 视图破坏 fork/export/回放 | 高 | store 测试无 marker 旧行为/原始保留/fork；e2e 下一轮 |
| 原始消息与压缩视图混淆 | 高 | marker `role=system` + id 前缀；UI 不展示 JSON；DB 原消息保留 |
| provider/网关失败/超时 | 中 | 失败不 append；可重试；usage/错误可读 |
| 超预算拆分 | 中 | 本 spec 12 文件独立验收 |

## 7. 已定决策（Lead 2026-11-01，D1/D2/D6 推荐 A/B）

- [x] **D1 compact 实现层级/history**（已定 A）：host `session.compact` + 专用 host summarizer（kit；personal provider / enterprise gateway，不产生 agent turn/tool 行/审批）；system marker 消息保存摘要/边界；**`messages_in` 原始语义不变**；新增 compacted view 仅供 replay/分隔 UI；原始消息保留；**不改 rig history 接口**；无 DB migration。
- [x] **D2 保留/预算/幂等**（已定 A）：最近 6 轮/12 条（取先到）；输入 ≤12k tokens、摘要 ≤2k tokens；失败/超限不写；无新消息 no-op；同一边界不重复写。
- [x] **D6 自动压缩**（已定 B）：本 spec 不做自动压缩；只显式 `/compact`，自动阈值另 spec。
- [x] **拆分**：本 spec 为 compact 子 spec；plan-mode/skill-commands 另两份；串行顺序 compact → plan-mode → skill-commands，共享文件由先实现者写。

## 8. 变更预算与拆分

- **12 文件**（≤12；**Lead 2026-11-01 书面批准 12 文件预算例外**）：契约 1（`packages/contracts/ipc/src/lib.rs`）；kit 2（`crates/zk-kit-compaction/{Cargo.toml,src/lib.rs}` 新）；host 1（`apps/desktop/src/ipc.rs` dispatch/编排）；ui-chat 2（`AssistantComposer.tsx`、`CompactDivider.tsx` 新）；app 3（`App.tsx`、`state/controller.ts`、`i18n.tsx`；protocol 合并进 App）；app 测试 1（`apps/desktop/web/tests/command-compact.test.tsx` 新）；e2e 2（`e2e/m1-serve.spec.ts`、`e2e/interaction/command-compact.spec.ts`）；合计 12。**Lead 2026-11-01 书面批准 12 文件预算例外**；docs/19/26 由 compact 首写（不重复计后续）。
- 生成物由生成器写；不新增 capability；无 DB migration。

## 9. 复用与已有实现（C17）

- 已搜索：`rg -n "messages_in|HistoryMessage|run_stream_in_shared_with_history|compact|summar" packages apps docs`。
- 复用：Batch1 菜单/catalog/runCommand；`SessionStore::messages_in`/`append_message`；agent-rig history 回放（`docs/38` B8）；provider/gateway 凭证（`zk_composed::gateway_credentials`）。
- 新增：`zk-kit-compaction`（host summarizer 单实现）；`CompactDivider` UI；不新增 agent turn/工具。
- 为什么不能复用：rig 只裁剪不摘要；rig-memory summarizer 未接/不可控；agent turn 摘要会污染会话。

## 10. 文档与决策

- D1/D2/D6 已定（Lead 2026-11-01）；实现前 C19/C20 评审。
- 回写 `docs/19`（compact 分隔/失败）、`docs/26`（IPC/错误码/marker/compacted view）、`KNOWN-LIMITATIONS`（无自动压缩/原始消息仍占 DB）；Lead 12 文件预算例外已记录。
