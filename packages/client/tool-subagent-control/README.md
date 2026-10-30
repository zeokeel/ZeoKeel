# tool-subagent-control (`tool.subagent_control`)

**功能说明**：`list_agents` 列出同步/异步子代理、深度与并发限制；`extra_tools()` 追加 `subagent_status {id}`（状态 + 最终结果）与 `subagent_interrupt {id}`（取消运行中的 async 子代理）。同步 runner 没有 async 注册表时，status/interrupt 返回明确错误（`unknown subagent`），不会假装成功。
