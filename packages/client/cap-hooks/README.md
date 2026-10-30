# cap-hooks (`agent.hooks`)

**功能说明**：Agent lifecycle hooks 能力。`zk-agent-rig` 实现 TurnStarted/TurnCompleted（含 usage）/
ToolStarted/ToolFinished/ApprovalRequested/ApprovalResolved/MemoryUnavailable/TurnFailed 事件，host 通过
`RigAgent::with_hook_sink` 或 JSONL（`agent.hooks.log`）观测。

- `available()` **动态**：默认 true（shipped agent 支持 hooks）；`ZEOKEEL_AGENT_HOOKS_ENABLED=0` 或
  `set_available(false, reason)` 时为 false。
- 不可用原因经 `unavailable_reason()` 返回；host 统一映射 `CAPABILITY_UNAVAILABLE`。
