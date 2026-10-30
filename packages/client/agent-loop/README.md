# zk-agent-loop

本地最小 agent 循环（包 id `zeokeel/agent-loop`，角色 `agent.loop`，契约 `zk-contract-agent`）。

- 实现 `Agent`；只依赖 `zk-contract-agent` / `zk-contract-tool` / `zk-contract-llm`，不认识具体实现。
- `run(input)`：组 prompt（`USER_INPUT:` + 单行工具指令提示）→ `LlmProvider::complete`；
  - provider 失败 → `AgentError::Llm`；
  - 输出以 `TOOL ` 开头 → 解析 `<tool> <args>`，只支持 `fs.read`；不支持或未注册 → `AgentError::Tool`；
    调用结果（成功或失败文本）都记入一条 `ToolCallRecord`，`AgentReply.text` 含结果或错误；
  - 否则原样返回 `AgentReply{ text, tool_calls: [] }`。
- 工厂：`agent(tools: Vec<Arc<dyn Tool>>, llm: Arc<dyn LlmProvider>) -> AgentLoop`（生成 glue 约定）。

```bash
cargo test -p zk-agent-loop --offline
```

替换 provider（mock → DeepSeek/Ollama）或工具（tool.fs → sandbox）不需要改本包。
