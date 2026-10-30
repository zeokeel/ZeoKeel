# zk-tool-fs

最小只读文件工具（包 id `zeokeel/tool-fs`，角色 `tool.fs`，契约 `zk-contract-tool`）。

- 实现 `Tool`：`spec().name == "fs.read"`；`call(args)` 的 `args` 是相对路径（自动 `trim`）。
- 安全边界（demo 级）：拒绝绝对路径、含 `..` 的路径；`canonicalize` 后必须仍在 root 内（防软链接逃逸）；
  只读、只读普通文件、≤64KiB；不存在 → `ToolError::Io`；越权 → `ToolError::Denied`；
  空参数/超限/非 UTF-8/非普通文件 → `ToolError::InvalidArgs`。
- 工厂：`tool()`（root = 环境变量 `ZEOKEEL_DEMO_ROOT`，否则当前目录）；`FsTool::new(root)` 供测试注入。
- 生成 glue 约定（M0）：`tool()` 导出本角色实现。

```bash
cargo test -p zk-tool-fs --offline
```

测试在 `tests/conformance.rs`（`cargo test` 专用 allow 见文件头）。
