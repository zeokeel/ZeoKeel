# tool-agent-team (`tool.agent_team`)

**功能说明**：父 agent 作为 coordinator，按 `agent_team {objective, tasks[], max_workers?, timeout_ms?}` 派发最多 8 个 worker
子代理（复用同一 model/tools、子会话隔离），等待并聚合每个 worker 的 `status`/`result`/`error` 后返回父 agent；
不做动态团队 UI、不做 LLM 综述（父 agent 自行综合）。

- **上限**：单次最多 8 个 tasks；`max_workers` 1..=8（默认 = tasks 数）；worker 并发与 `subagent_start` 共享
  runner 的存活线程上限 8；委派深度沿用 `MAX_DEPTH=3`（深层嵌套时先经 `current_depth()` 拒绝）。
- **聚合**：同步有界等待（轮询 runner 的 `status`/`result`，50ms 间隔），返回
  `{objective,status,workers[{id,task,status,result|error}],completed,failed,cancelled,total,max_workers,timeout_ms,depth,elapsed_ms}`；
  `status` ∈ `done|partial|failed|timeout`。单个 worker 失败不使整次调用失败（`partial`），全部失败为 `failed`。
- **错误**：runner 未安装 / 深度超限 / worker 并发上限已满 → `Denied` 明确错误；启动失败 → 中断已启动 worker 并返回
  `Io`（含失败 task id）；越界参数 → `InvalidArgs`。
- **超时/取消**：`timeout_ms` 默认 60000、范围 100..300000；到时中断仍在运行的 worker 并返回 `status:"timeout"`。
  父轮次取消由运行时在工具边界生效（工具自身等待有界，建议 `timeout_ms` 低于 `AppSettings.tools.tool_timeout_ms`）。
- 决策见 ADR-070（`docs/13`）；行为测试用确定性 fake runner（无网络）：成功/部分失败/全失败/超时中断/cap/深度/启动失败/参数校验。
