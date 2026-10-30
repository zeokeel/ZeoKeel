# zk-agent-rig (`agent.loop`)

**功能说明**：用 `rig-agent`/`rig-core` 作为唯一 agent 内核。工具以 rig `DynamicTool`（JSON Schema）注册，支持多步工具调用、流式事件、取消与最大步数；不自研第二套 loop。提示词来自 DSH 参考（见 docs/33-agent-prompts）。
