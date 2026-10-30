# 2026-10-31-config-and-media · 提案

- 状态：`approved`
- Owner：lead（auditor-rig / auditor-web / fixer-media）
- 日期：2026-10-31
- 关联：ADR-066；docs/38 §5；上一轮 acceptance backlog §3.3/3.4
- 评审：docs/reviews/2026-10-31-round2-verification.md
- 变更预算：契约配置 + 桌面/设置 UI + cap-* 媒体能力；按域提交

## 1. 背景 / 问题

用户核心目标要求"所有项目均可配置、rig 所有能力都支持"。上一轮已完成 rig 默认发货、completion 配置 API、usage/memory/hooks、embeddings/vector；但：
- `AppSettings` 还没有 `agent` 配置段，设置页无法配置 rig 参数/内存/hooks；桌面启动只配置 provider，未把 agent 配置应用到 rig。
- cap-image/audio/transcription/rerank 仍是"诚实不可用"占位，rig-core 已提供对应 provider 模块（image/audio/transcription/rerank），但未实现。
- usage 已采集在 agent-rig `last_usage()`，但未进入契约/桌面展示。

## 2. 目标

- `AppSettings.agent`（max_steps/completion/memory/hooks）契约 + serde default；桌面启动与保存时应用到 agent-rig；设置页可编辑并持久化。
- `AgentReply.usage`（含 cached/reasoning/cost）契约 + agent-rig 填充；桌面消息视图展示 token 用量。
- cap-image/audio/transcription/rerank 用 rig-core 对应模型实现（仅个人版可配置 provider；企业网关不提供这些模态时保持诚实不可用）。
- T22 前端：桌面边界用生成 `SCHEMAS` + Ajv 校验（至少 IPC 响应/UiEvent/AgentEvent/Settings），删除对应的手写字段 guard。

## 3. 非目标

- 企业网关 embeddings/media 路由（需要 ADR，网关只允许 Responses）。
- 系统密钥库、媒体资产存储/流式下载管线。
- 桌面 Ajv 覆盖全部 53 schema（先覆盖边界事件/设置；其余后续）。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 个人用户 | 设置 temperature/max_tokens/tool_choice/memory/hooks | 保存后下一轮生效，重启保持 |
| 个人用户 | 需要图片/语音/转写/rerank | 配置 provider 后能力可用，否则明确不可用原因 |
| 开发者 | 边界数据校验 | 非法 IPC/事件被 Ajv 拒绝并记日志，不靠类型断言 |

## 5. 成功指标

- `AppSettings.agent` schema/TS 生成且 schema-check 绿；设置保存后 agent-rig 参数生效（测试断言 rig 请求参数）。
- 桌面 Ajv 负向测试：非法 CommandResponse/UiEvent/AgentEvent 被拒绝；合法数据通过。
- 4 个媒体 cap 单元测试可配置/不可用路径；docs/38 矩阵更新。
- 全量门禁与既有 e2e 保持绿。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解 |
| --- | --- | --- |
| rig 媒体 feature/依赖 | 构建失败 | 只启用 cached feature（audio/image/pdf/epub），本地验证 |
| 设置 UI 与契约漂移 | 运行期配置无效 | schema 生成 + schema-check + 单测 |
| Ajv 迁移面大 | 回归 | 先覆盖边界 envelope/事件/设置，保留现有安全 reader 作二次校验 |

## 7. 需要用户拍板

- 无阻塞；媒体 provider 仅个人版自配，企业网关不支持媒体（C7 Responses-only）。
