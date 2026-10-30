# 2026-10-30-agent-dsh-parity · 任务清单

| # | 任务 | 写作用域 | 依赖 | 验证 | 状态 |
| --- | --- | --- | --- | --- | --- |
| 1 | DSH 清单 + prompt 提取文档 | `research/dsh-agent-inventory.md`, `docs/39-agent-prompts.md`, `docs/generated/agent-tool-matrix.md` | — | inventory diff；docs-check | ✅ |
| 2 | rig 依赖/契约/模型 provider（OpenAI-compatible + fake model） | `Cargo.toml`, `packages/contracts/rig`, `packages/client/llm-rig` | 1 | cargo tests；fake model tools | ✅ |
| 3 | Tool 契约 JSON Schema/ToolResult + kit | `packages/contracts/tool`, `crates/zk-kit-tools` | 1 | conformance/负向 | ✅ |
| 4 | rig Agent runtime/提示词/事件/取消/最大步数/hooks | `packages/client/agent-rig`, `packages/contracts/agent` | 2,3 | rig mock multi-step e2e | ✅ |
| 5 | 审批 gate + IPC/UI | `apps/desktop`, `packages/ui/ui-chat`, `ui-approval` | 4 | Playwright 批准/拒绝 | ✅ |
| 6 | P0 工具包（fs read/write/edit/search、bash、todo、ask-user、present、working-directory、web.fetch） | `packages/client/tool-*`, kit | 3 | 每工具单测+负向 | ✅ |
| 7 | P1 工具包（persistent/terminal/skill/schedule/jobs/goal/session-query/deps/call-timeout/web.search） | 同上 | 6 | 单测+集成 | ✅ |
| 8 | P2 工具包（subagent/workflow/ralph/worktree/pwsh/tool-cordis/agent-team） | 同上 | 7 | 单测/平台负向/预算 | ✅ |
| 9 | personal provider 设置运行时生效 | `apps/desktop`, `llm-openai-compatible` | 2 | 设置→聊天 fake upstream | ✅ |
| 10 | 全量 e2e/a11y/矩阵/验收 | e2e, scripts, acceptance | 1–9 | Playwright + cargo + 独立评审 | ✅ |

## 2. 写作用域

- 只写：契约/agent-loop/tool 包/kit/desktop/UI/生成矩阵/docs/research/e2e；不改企业服务端业务语义（gateway tools 兼容除外）。
- 生成物由 `zk compose`/脚本写。

## 3. 验收

- 工具矩阵 diff=0；全部工具正/负测试；多步/审批/取消 e2e；个人 provider 设置生效；独立评审 APPROVED。
