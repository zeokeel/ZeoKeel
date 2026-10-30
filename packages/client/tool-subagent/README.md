# tool-subagent (`tool.subagent`)

**功能说明**：把任务委派给子 agent；复用父 agent 的 rig model/tools，限制最大深度 3，子 agent 不直接向前端发事件。

**异步**：同一包通过 `extra_tools()` 追加 `subagent_start {prompt}`（后台启动，立即返回 `{id}`）；配合 `tool-subagent-control` 的 `subagent_status`/`subagent_interrupt` 使用。并发上限 8（runner 级），取消通过共享 `Arc<AtomicBool>` 传播；不支持异步的 runner 返回 `async subagents are not supported`。
