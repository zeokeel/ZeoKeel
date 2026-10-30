# 2026-10-30-m0-scaffold-minimal-demo · 技术设计

> 规范细节链接 `docs/`；组合/manifest 以 `24` 为准，本文件只写本次落地。

## 1. 影响面

| 类型 | 变更 |
| --- | --- |
| 包 | contracts: package/kernel/tool/llm/agent；client: tool-fs / llm-mock / agent-loop |
| 契约 | 5 个最小契约（trait + DTO + 错误）；无破坏性变更 |
| 路由 | server 仅 `/healthz` `/readyz` `/api/meta` |
| 迁移 | 无（本 demo 不接 DB） |
| 生成物 | `crates/zk-composed/*`、`apps/*/Cargo.toml`、`apps/*/src/generated/packages.rs`、`shared/contracts/{packages,roles,capability-index}.json`、`docs/generated/{PACKAGE-INDEX,CAPABILITY-INDEX}.md` |
| edition/profile/channel | `personal` × `default` × `official`；enterprise 清单占位 |
| 文档 | `10`/`12`/`24` 的 M0 子集落实；spec 归档 |

## 1.5 复用检查（C17，必填）

| 要实现的点 | 现有能力 | 结论 | 归属 |
| --- | --- | --- | --- |
| 工具调用协议 | 无（首次实现） | 新建 | `zk-contract-tool` + `tool-fs` |
| LLM 调用 | 无 | 新建 | `zk-contract-llm` + `llm-mock` |
| agent 循环 | 无 | 新建 | `zk-contract-agent` + `agent-loop` |
| TOML 解析/CLI | 标准库 + `toml` crate | 复用依赖 | `tools/zk` |
| HTTP 健康检查 | axum | 复用依赖 | `apps/server` |
| 桌面窗口 | tauri 2 | 复用依赖 | `apps/desktop` |
| 共享 helper | 暂不需要；不建 `utils` | — | — |

- 搜索证据：本仓库无产品代码（`git ls-files` 仅 docs/specs/experiments），能力索引为空，故本批为首次实现。
- 新增能力：`contract.tool`、`contract.llm`、`contract.agent`、`tool.fs`；已用 `@capability` 注释登记。

## 2. 方案

```text
zk.toml + editions + profiles + channel.json
        │  zk validate / compose
        ▼
crates/zk-composed (生成 glue: PACKAGES/EDITION + build() -> Runtime)
        │                         ├─► apps/desktop (Tauri) ── invoke ipc_call("demo.chat")
        │                         └─► apps/server  (axum)  ── /healthz
        ▼
contracts (tool/llm/agent/package/kernel) ◄── packages (tool-fs/llm-mock/agent-loop)
```

### 2.1 关键流程（demo turn）

```text
用户输入 → agent-loop: 组 prompt → mock LLM 返回 "TOOL fs.read <path>"
        → agent-loop 查工具表 → tool-fs 读取（路径限制）→ AgentReply{text, tool_calls}
        → Tauri ipc_call 返回 JSON → HTML/JS 渲染
```

### 2.2 数据结构

- `ToolSpec{name,description}`、`ToolError{NotFound,Io,Denied,InvalidArgs}`。
- `AgentReply{text, tool_calls: Vec<ToolCallRecord>}`、`ToolCallRecord{name,args,result}`。
- `PackageDescriptor{id,version,kind,roles}`。
- 生成物：`Runtime{agent: Arc<dyn Agent>, packages: &'static [&'static str]}` + `build() -> Result<Runtime,String>`。

### 2.3 解析/生成规则（zk v0）

- 扫描 `packages/*/*/zk.toml`；忽略 `experiments/`。
- editions：`personal` 基线；`enterprise` `base_edition=personal` + exclude（本 demo 仅占位）。
- profiles：base 链 + `[packages]` 覆盖 singleton + `[packages.add]/remove`（本 demo 未用 add/remove）。
- 角色注册表：根 `zk.toml [roles]`；契约映射：根 `zk.toml [contracts]`。
- glue 构造约定（本 spec 限定）：`tool.*` 包导出 `tool() -> Arc<dyn Tool>`；`llm.provider.default` 导出 `provider()`；
  `agent.loop` 导出 `agent(tools, llm)`。M0 后续由 Registrar/fiber 取代。
- `--check`：先在内存生成，与磁盘逐字节比较；不写。

## 3. 备选与取舍

| 方案 | 优点 | 缺点 | 结论 |
| --- | --- | --- | --- |
| Tauri 2 + 静态 HTML | 真桌面窗口；依赖已缓存 | 首次编译慢 | 采用 |
| serve 载体 + 浏览器 | 无系统依赖 | 不是最终桌面形态 | 降级备选（Tauri 失败时） |
| 完整 Registrar/fiber | 贴近最终架构 | 本 demo 工作量过大 | 推迟到 M0 后续 |
| React/Vite | 贴近最终 UI | 需 pnpm 安装/构建 | 推迟到 M1 |

## 4. 安全与隐私

- tool-fs 限制：仅相对路径、拒绝 `..`/绝对路径、只读、≤64KB、根目录 `ZEOKEEL_DEMO_ROOT` 或 cwd。
- demo 无密钥、无网络、无遥测；mock LLM 不调用外部 API。
- Tauri CSP 先 `null`（仅本地静态页面）；M0 后续收紧。

## 5. 发布与回滚

- 无发布；本地 `cargo run -p zk-desktop` / `-p zk-server`。
- 回滚：删除生成物并由 `zk compose` 重建；无数据/迁移影响。

## 6. 测试计划

| 层 | 用例 | 命令 |
| --- | --- | --- |
| T0 | validate / compose --check / capability --check 可复现 | `cargo run -p zk -- validate` 等 |
| T1 | tool-fs 越权/读写；llm-mock 决策；agent-loop 工具调用；契约 conformance | `cargo test -p zk-tool-fs -p zk-llm-mock -p zk-agent-loop` |
| T2 | composed build + demo 链路（mock） | `cargo test -p zk-composed` |
| 服务端 | `/healthz` 200 | `cargo test -p zk-server` 或 curl |
| 文档 | docs/spec 门禁 | `python3 scripts/*-check.py` |

## 7. 文档更新

- [x] `docs/10` 布局与 `docs/12` M0 子集（已存在，本次按此落地）
- [x] 新主题：无（沿用 10/12/24/25）
- [ ] `docs/generated/*` 由生成器产出，不手写
- [x] spec 完成后归档（`specs/archive/`）

## 8. 风险

| 风险 | 概率 | 影响 | 缓解 | 验证 |
| --- | --- | --- | --- | --- |
| Tauri 编译/链接失败 | 中 | 桌面 demo 不可用 | 先 check；失败降级 serve 并记录 | `cargo check -p zk-desktop` |
| 生成物与 app Cargo 冲突 | 中 | 构建失败 | 模板单一来源 + `--check` | `zk compose --check` |
| 简化 glue 约定 | 高 | 后续返工 | spec 非目标 + M0 后续接 Registrar | 评审 |
| 离线依赖缺 | 低 | 构建失败 | 只用缓存 crate | `--offline` |
