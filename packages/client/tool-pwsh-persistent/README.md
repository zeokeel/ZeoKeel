# tool-pwsh-persistent (`tool.pwsh_persistent`)

**功能说明**：**已实现**的长驻 PowerShell 7 会话（与 `terminal` 相同的多会话模型：`open`/`status`/`send`/`close`/`list`，
session 默认 `"default"`）。`open`/`send` 通过 `zk-kit-tools` 启动 `pwsh -NoProfile -NonInteractive -Command -`，cwd/env
跨调用保留；`send` 前必须先 `open`。**平台门控（诚实不可用）**：`pwsh` 不存在（如多数 Linux/macOS 主机）时返回
`TOOL_UNSUPPORTED_PLATFORM`，不假装成功；Windows/pwsh 7 环境走真实会话。审批语义不变（mutating + requires_approval）。

**测试**：`tests/behavior.rs` 在 Linux 上覆盖诚实不可用（TOOL_UNSUPPORTED_PLATFORM）与未知 session/非法参数；
在装有 pwsh 的主机上覆盖真实 open/send/close。`tests/conformance.rs` 校验包元数据。
