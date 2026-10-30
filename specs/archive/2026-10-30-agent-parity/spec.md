# 2026-10-30-agent-dsh-parity · 规格

- 状态：`approved`
- 提示词来源：DSH `/usr/local/lib/node_modules/@deepseek-ai/dsh/node_modules/@deepseek-ai/*`；仅 MIT 文本/语义，不依赖运行时。

## 0. 内核

- `rig-agent` AgentBuilder/Agent 是唯一 agent runtime；工具是 rig DynamicTool（JSON Schema + callback），不是字符串协议。
- rig 能力包矩阵：`agent.rig`（run/stream/hooks/run state）、`llm.provider.rig`（OpenAI/DeepSeek/Anthropic 等 rig provider）、`memory.rig`（rig-memory conversation persistence）、`extractor.rig`（structured output/extraction）、`vector.rig`（dynamic context/vector store）、`loader.rig`（PDF/EPUB/text loaders）、`tool.rig`（DynamicTool 注册）。每项独立包/角色，personal 可选，enterprise 只启用不违反 C7 的 provider。
- 任何 rig 能力未配置/平台不支持时返回明确 `CAPABILITY_UNAVAILABLE`，不得静默缺失。

## 1. LLM 契约

- `LlmMessage { role: system|user|assistant|tool, content, tool_call_id?, tool_calls[] }`
- `ToolSchema { name, description, parameters(JSON Schema) }`
- `LlmRequest { messages, tools, temperature, max_tokens }`
- `LlmResponse { content, tool_calls[{id,name,arguments_json}], usage? }`
- `LlmProvider` 仅作为兼容门面；真实模型通过 rig `DynModel<Completion>` 注入 agent。自研 provider 必须适配为 rig `CompletionModel`，不再自研 loop/工具协议。
- personal OpenAI-compatible：`/chat/completions` tools/tool_choice/stream；企业 gateway：Responses tools 兼容子集，不支持则返回明确错误而非静默丢工具。

## 2. Agent loop

- 系统提示：DSH identity（"You are an AI agent powered by DeepSeek Harness." 出处）+ ZeoKeel 运行事实 + workspace AGENTS.md/CLAUDE.md 指令 + 工具使用规范 + 当前日期/workspace/cwd；DSH 不可提取的 persona 用配置替代并记录。
- 循环：最多 50 步；每步模型返回 text/tool_calls；tool_calls 按 DSH 语义执行（本实现先顺序，标记并发后续）；结果作为 `tool` message 回填；无 tool_calls 结束。
- 审批：`ToolSpec.requires_approval`；命中时事件 `ApprovalRequested`，agent 暂停等待 `ApprovalGate` 决定；拒绝则回填 denied 结果；取消事件终止。
- 错误：未知工具/无效 JSON/工具错误回填为 tool result，不使整轮崩溃；达到步数上限返回 `AGENT_MAX_STEPS`。
- 事件：TextDelta、ToolCallStarted/Finished、ApprovalRequested/Resolved、TurnCompleted/Cancelled/Error；UI reducer 兼容。

## 3. 工具契约与包

- `ToolSpec { name, description, parameters, requires_approval, read_only }`；`Tool::call_json(ctx, args: Value) -> ToolResult { content, is_error, metadata? }`。
- 每个 DSH 工具模块一个 ZeoKeel 包（`tool.<name>` role，`packages/client/tool-<name>`，crate `zk-tool-<name>`，kit 复用 `zk-kit-tools` 仅用于通用路径/进程/结果封装）。
- 必须实现矩阵（以 DSH 包清单为准）：fs.read/write/edit/search、bash、bash-persistent、pwsh、pwsh-persistent、terminal、web.fetch、web.search、todo、ask-user、present、skill、schedule、jobs、goal、ralph、subagent、subagent-control、workflow、worktree、session-query、call-timeout、working-directory、workspace-dependencies、tool-cordis（禁用/替代说明）、agent-team（experimental）。
- 平台不支持的工具（如 pwsh 无 PowerShell）注册但调用返回 `TOOL_UNSUPPORTED_PLATFORM`，测试覆盖；不得静默缺失。
- 所有工具限制在工作区内（除显式 `allow_outside` 开关，默认 false）；bash/pwsh 有超时/输出上限；web 有 SSRF 白名单/大小限制；subagent/workflow 有深度/预算限制。

## 4. 个人版 provider/设置

- 设置保存后重建 provider（无需重启）；API key 仍本地 0600（M1 过渡），读取掩码；provider 选择支持 openai-compatible/mock；model/base_url 校验。
- 工具调用真实模型 e2e 用本地 fake upstream（Chat Completions tools），验证多步。

## 5. 验收断言

- 工具矩阵文件（`docs/generated/agent-tool-matrix.md`）与 DSH 包清单 diff=0；每工具至少 1 正/1 负测试。
- `cargo test --workspace`/clippy；agent mock e2e：多步 fs→edit→bash、审批拒绝、取消、最大步数、未知工具。
- Playwright：真实 UI 发请求→工具卡→审批→继续/拒绝→结果；设置 provider→保存→聊天使用新配置（fake upstream）；axe 0。
- 独立评审 APPROVED。
