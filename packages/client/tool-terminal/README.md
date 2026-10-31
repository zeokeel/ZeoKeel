# tool-terminal (`tool.terminal`)

**功能说明**：当前 workspace/conversation 的**多会话持久终端**。`open`/`status`/`send`/`close`/`list` 以 `session`
（默认 `"default"`）寻址；每个会话独立保留 cwd/env，`send` 前必须先 `open`（未知 session 明确报错）。session id
限 ASCII 字母/数字/`-_.` 且 ≤64 字节，每会话每种 shell 上限 8 个；无 secret/网络/路径语义变化，审批语义不变。

**测试**：`tests/behavior.rs` 覆盖默认会话、两个会话隔离、关闭一个不影响另一个、未知 session、非法 session id、
未知 action；`tests/conformance.rs` 校验包元数据。
