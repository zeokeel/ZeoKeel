# 38 · Agent runtime（rig）

> 真源：`agent.loop` = `packages/client/agent-rig`，模型 = `llm.provider.default`，工具 = `tool.*` 包。DSH 只作参考（`research/dsh-agent-inventory.md`），不运行 DSH。

## 1. 内核

- `rig-agent` AgentBuilder/Agent 是唯一循环：preamble、DynamicTool（JSON Schema）、max turns、hooks、extractor、memory/vector 由 rig 提供。
- 每个工具是独立包（`zk-tool-*`），角色 `tool.<名>`；共享路径/进程/截断逻辑只在 `crates/zk-kit-tools`（C17）。
- 工具通过 `TurnContext` 获得 workspace/conversation；DynamicTool 回调闭包捕获同一 context，禁止把工具绑定到全局 cwd。
- 提示词：DSH identity + persona（`You are an AI agent powered by DeepSeek Harness.` / `You are a coding agent powered by the … model.`）+ 工作区 `AGENTS.md`/`CLAUDE.md`（≤64KB）+ 工具说明。来源见 `research/dsh-agent-inventory.md` §3。

## 2. 审批

- `ToolSpec.requires_approval` 的工具在 rig DynamicTool 回调内暂停：发 `ApprovalRequested` → `ApprovalGate.decide` 阻塞 → 发 `ApprovalResolved` → 继续/拒绝。
- 默认 `DenyAll`（fail-closed）；桌面在 `AppState` 安装 `ApprovalBroker`，UI `approval.decide` 解除等待；超时/无通道 = 拒绝。
- 这是 DSH `ask` 的等价语义；sandbox 升级不在弱管控范围。

## 3. 事件/取消/上下文

- `EventSink` 通过 `run_stream_in_shared(Arc<dyn EventSink>)` 传入，工具与审批可在执行中发事件；旧 `&dyn` 路径用缓冲 sink 兼容。
- 每轮事件：TurnStarted → ApprovalRequested/Resolved → ToolCallStarted/Finished → TextDelta → TurnCompleted/Cancelled/Error。
- 取消：rig 流式循环每项检查 `RigAgent.cancel()`；工具执行前检查取消；桌面 `chat.cancel` 调用 `Runtime::cancel_agent()`。工具超时可由 `ZEOKEEL_TOOL_TIMEOUT_MS` 配置（默认 120s）。非流式 provider 通过 50ms 轮询取消标志也可中断；审批超时由 `ZEOKEEL_APPROVAL_TIMEOUT_MS` 配置（默认 300s）。
- 委派：subagent/workflow/ralph 复用同一 model/tools，但子会话 id 隔离为 `<conversation>-sub-<depth>`；最大深度 3。

## 4. 工具矩阵

- 已实现：`fs.read`、`fs.write`、`fs.edit`、`fs.search`、`bash`、`bash_persistent`、`job_output/list/kill`、`glob`、`grep`、`skill`、`web.fetch`、`working_directory`、`todo`、`present`、`load_workspace_dependencies`、`create_worktree`、`terminal`、`goal`、`schedule`、`pwsh`、`read_image`、`subagent`、`subagent_fork`、`workflow`、`ralph`、`ask_user_question`、`extract_json`、`read_document`（PDF/EPUB）、`session_search`、`web_search`、`pwsh_persistent`、`subagent_control`、`cordis`（禁用）、`agent_team`（实验禁用）；UI 已有审批卡、问题卡、provider 流式/取消；每个工具包至少一条行为测试，`scripts/agent-e2e.sh` 覆盖 9 条人机流程（审批/拒绝/取消/ask-user/bash/present/多步；retries=0）。
- 部分/未实现：`agent-team`（未启用）、`call-timeout`（固定 120s 策略）、schedule 执行器、workflow 并行（DAG 已支持）、subagent fork/异步 control、pwsh persistent 平台支持。
- 完整矩阵与测试状态以 `docs/generated/AGENT-TOOL-MATRIX.md` 为准。
- 平台不支持必须返回明确错误，不得静默缺失。

## 5. rig 能力释放

- 已用：AgentBuilder、DynamicTool、preamble、max turns、streaming（text delta）、取消标志、审批阻塞、工具超时。
- 已接：rig-memory（`ZEOKEEL_AGENT_MEMORY_DIR`）、rig extractor（`extract_json` 工具）。
- 已接多 provider：openai/deepseek/anthropic/ollama（`ZEOKEEL_RIG_PROVIDER`）。
- 设置保存后通过 `RigAgent::configure_model` 热切换 rig model（无需重启）；单测 `configure_model_hot_swaps`。
- 待接：hooks 全面观测、extractor、vector store、loaders、图像/音频/转写/rerank。
- 每项独立能力包/角色；未配置时 `CAPABILITY_UNAVAILABLE`。
