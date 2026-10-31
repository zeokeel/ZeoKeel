# tool-goal (`tool.goal`)

**功能说明**：会话级目标 + 有界 round driver（spec R9.1 / F3）。状态持久化在 `.zeokeel/goal.json` 的
`goals.<conversation_id>`（旧版单目标文件在读取时自动迁移）。

- `set`/`create`：设置 objective（≤4096B）与 `max_rounds`（默认 10，clamp 1..=100），清零 rounds、status=active。
- `status`/`get`：返回当前会话目标 JSON（无目标返回 `null`）；`update` 保留旧字段（status/reason/rounds）兼容。
- `run`：**每次调用只执行一轮** agent turn；达到 achieved / cancelled / max_rounds 即停止，不会无限循环。
- `cancel`：取消目标；保留 rounds 与 round_log 供检查。
- 持久化语义：轮次在 turn 执行**前**写入（`rounds` + `in_flight_round`），成功/失败后更新；崩溃/重启后下一次 `run`
  重试同一轮（至多执行一次、不重复计数）。每轮在 `round_log` 记录 result/achieved/usage（有则记录）。
- 运行时：host 可通过 `driver::set_turn_runner(Some(Arc<dyn GoalTurnRunner>))` 注入 agent turn；未注入时使用已安装的
  `zk-kit-subagent` runner 执行一轮子 agent turn（无需改 agent-rig）；两者都没有时 `run` 显式报
  `goal round runner unavailable` 且不消耗轮次。子 agent 输出精确行 `GOAL_ACHIEVED` 视为目标达成。

**测试**：`tests/tool.rs` 用确定性 fake runner 覆盖单轮/上限停止、achieved 停止、cancel、崩溃后恢复同一轮、
runtime 缺失显式错误、legacy create/update 与 clamp、会话隔离。
