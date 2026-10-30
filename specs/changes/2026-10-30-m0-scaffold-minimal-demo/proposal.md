# 2026-10-30-m0-scaffold-minimal-demo · 提案

- 状态：`approved`
- Owner：lead（AI）
- 日期：2026-10-30
- 关联：D1–D65 / ADR-001…057 / `docs/10`、`docs/12` M0、`docs/25`
- 变更预算：约 35 个文件 / 2000 行；按任务表分步提交

## 1. 背景 / 问题

`docs/` 已到 Plan v3.2，仓库只有文档、研究、两个实验原型，**没有产品代码**。用户直接要求：
“现在就开始把整个项目的骨架搭建起来。并且实现一个最小的 demo”。

骨架必须证明三件事：单一 workspace 能编译；`zk` 能在构建期换包并生成 glue；两端（桌面/服务端）能启动。
最小 demo 必须跑通一条真实链路：**客户端 UI → 本地 agent → 工具调用 → 返回结果**。

## 2. 目标

- 单一 Rust workspace（contracts / client packages / apps / tools）+ `zk.toml` / editions / profiles / brands。
- `zk` v0：`validate` / `compose [--check]` / `plan` / `capability [--check]` / `find` / `lint`（路径与重复的 M0 占位）。
- 最小契约集：package / kernel / tool / llm / agent（含 DTO、错误、符合性测试入口）。
- 最小包集：`tool.fs`、`llm.provider.default`、`agent.loop`，含 conformance 测试。
- `crates/zk-composed`（生成 glue）+ 两端 app（Tauri 桌面 + axum 服务端）。
- demo：桌面窗口输入“读取 demo-data/hello.txt” → mock LLM 决策 → `tool.fs` 执行 → 显示正文 + 工具调用记录。
- 服务端 `/healthz`、`/readyz` 返回 edition / 包清单。
- `docs-check`/`spec-check` 0/0；`cargo test` 绿；`zk compose --check` 可复现。

## 3. 非目标

- 完整 M0 的 cordis fiber / Registrar / 换包矩阵 / CI 全矩阵（后续 M0 spec）。
- 服务端 DB、认证、钱包、网关、管理台、WASM、OEM、更新。
- React/Vite 前端工程（本 demo 用静态 HTML+JS，M1 换 React 19）。
- 真实 LLM provider（仅 mock；DeepSeek/OpenAI/Ollama 在 M1）。
- 完整 `zk` 语义（`zk.lock`、`cargo metadata` 单实例校验、db lock、docs 生成）——先做 demo 必需子集，其余登记为后续任务。

## 4. 用户与场景

| 用户 | 场景 | 期望 |
| --- | --- | --- |
| 开发者 | clone 后 `bash scripts/bootstrap.sh` / `cargo test` | 一条命令构建、测试全绿 |
| 演示者 | 启动桌面 app，输入“读取 demo-data/hello.txt” | 看到文件内容 + 工具调用记录 |
| 架构评审 | 改 `editions/personal.toml` 换工具实现 | `zk compose` 生成新 glue，构建期换包生效 |
| 运维 | 启动 server，curl `/healthz` | 200 + edition/包清单 |

## 5. 成功指标

- `cargo test --workspace` 通过（契约/包/服务端单测 + demo 链路测试）。
- `zk validate` 通过；负向：缺角色/重复 singleton/未知契约均在 3 秒内失败。
- `zk compose --edition personal --profile default --channel official --check` 通过且重复执行字节一致。
- 桌面 demo 一次对话返回 `demo-data/hello.txt` 内容与 `tool_calls[0].name=fs.read`。
- `python3 scripts/docs-check.py` / `spec-check.py` 0 error。

## 6. 风险与未知

| 风险/未知 | 影响 | 缓解/验证 |
| --- | --- | --- |
| Tauri/webkit 构建依赖缺失 | 桌面 demo 起不来 | 先 `cargo check`；失败则降级为 `serve` 载体 + 浏览器，并记录到 acceptance |
| 缓存/离线依赖不足 | 构建失败 | 只用已缓存 crate；`cargo build --offline` 验证 |
| 简化 bootstrap（未接 Registrar/fiber）偏离最终架构 | 返工 | 本 spec 显式列为非目标；M0 后续任务接 Registrar |
| 生成物与手写文件互相覆盖 | 混乱 | 生成物全部带 `@generated` + `--check`；模板放 `tools/zk/templates/` |

## 7. 需要用户拍板

- [x] 直接开始骨架 + 最小 demo（用户 2026-10-30 对话指令，视为批准本范围）
- [ ] 后续：完整 M0（Registrar/fiber、换包矩阵、CI 全量）另建 spec

## 8. 变更预算与拆分

- 预计改动：约 35 文件（含生成物）/ 2000 行。
- 拆分：T1 spec（本目录）→ T2 workspace+契约 → T3 包 → T4 zk/compose → T5 apps/UI → T6 测试与证据。
