# 2026-10-30-agent-dsh-parity · 设计

- 状态：`draft`

## 1. 契约/包

- 新 `packages/client/agent-rig`（role `agent.loop`）用 `rig_agent::AgentBuilder` 构建；工具注册为 rig DynamicTool；system preamble 由 DSH 提示词模块生成；run/stream 事件映射到现有 AgentEvent。
- `packages/contracts/rig` 暴露 `RigModel = DynModel<Completion>` 与 `RigModelFactory`，避免契约依赖具体 provider。
- 保留旧 `llm` 契约仅给 enterprise gateway；personal 走 rig OpenAI 兼容 provider。
- 新 `crates/zk-kit-tools`：路径边界、输出截断、进程执行、结果渲染、JSON schema helper；工具实现尽量放各自包，通用逻辑进 kit（C17）。
- 每个工具包四件套；角色 `tool.<name>`；`zk.toml` editions personal+enterprise（企业版工具仍本地，符合弱管控）；personal/enterprise edition 注册；`zk compose` 生成 tools 列表。
- `docs/generated/agent-tool-matrix.md` 由 `zk capability`/脚本生成，列 DSH 包→ZeoKeel 包/角色/状态/测试。

## 2. 提示词

- `crates/zk-kit-agent`（或 agent-loop 内）`prompt.rs`：DSH identity；工具 section 按 schema 生成；workspace instructions 读取 AGENTS.md/CLAUDE.md（大小上限/去重/截断，DSH 语义）；环境事实（cwd/workspace/date/platform）；工具使用/审批/结果规则。
- 每条 prompt 注释来源文件/行；无法本地提取的 profile persona 在 docs/33-agent-prompts.md 标注替代。

## 3. Agent runtime

- `RigAgent { agent: rig_agent::Agent, tools: Vec<DynamicTool>, limits }`；每 workspace/conversation 构建或复用 Agent（rig-memory 持久会话）。
- preamble = DSH identity + workspace instructions + tool guidance + runtime facts；rig hooks 处理审批/取消/观察。
- streaming 用 rig `AgentRunner`/stream；工具调用由 rig 驱动，不手写 tool_calls 解析。
- 审批 gate：trait `ApprovalGate { fn decide(&self, request) -> bool }`；桌面实现通过 IPC channel + UI；默认拒绝。为不阻塞 UI 线程，Agent 在 worker thread 运行，gate 阻塞等待。
- 取消：`CancellationToken` 检查点；工具执行前后检查。
- 事件映射到现有 `AgentEvent`，扩展 approval resolved/tool metadata。

## 4. provider

- OpenAI-compatible：tools schema、tool_calls 解析（含 arguments JSON）、SSE delta 拼接；设置变更后 `set_config`。
- Gateway：Responses API function tools；若上游不支持，返回 `LlmError::InvalidRequest("tools unsupported")`。
- Mock provider：按脚本返回多步 tool_calls 用于测试。

## 5. 工具实现顺序（P0→P2）

P0：fs.read/write/edit/search、bash、todo、ask-user、present、working-directory、web.fetch。
P1：bash-persistent、terminal、skill、schedule、jobs、goal、session-query、workspace-dependencies、call-timeout、web.search。
P2：subagent、subagent-control、workflow、ralph、worktree、pwsh（平台）、tool-cordis（禁用说明）、agent-team。

## 6. 测试

- 每工具单测（边界/负向/workspace 逃逸/超时/输出上限）；agent 契约 conformance；mock e2e；Playwright 做人机操作。
- 独立子代理反证工具矩阵与测试假绿。
